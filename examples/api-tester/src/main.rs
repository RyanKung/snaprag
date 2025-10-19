use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

mod api;
mod payment;
mod wallet;

/// Handle payment flow: sign and retry request with payment header
async fn handle_payment(
    requirements: &payment::PaymentRequirements,
    account: &wallet::WalletAccount,
    api_url: &str,
    endpoint: &api::EndpointInfo,
    body: Option<String>,
) -> Result<api::ApiResponse, String> {
    // Generate nonce and timestamp
    let nonce = payment::generate_nonce();
    let timestamp = payment::get_timestamp();

    // Get payer address
    let payer = account
        .address
        .as_ref()
        .ok_or("No wallet address available")?;

    // Create EIP-712 typed data
    let typed_data = payment::create_eip712_typed_data(requirements, payer, &nonce, timestamp)?;

    // Sign with MetaMask
    let signature = wallet::sign_eip712(&typed_data)
        .await
        .map_err(|e| format!("Failed to sign payment: {}", e))?;

    // Create payment payload
    let payment_payload =
        payment::create_payment_payload(requirements, payer, &signature, &nonce, timestamp);

    // Encode to base64
    let payment_header = payment_payload
        .to_base64()
        .map_err(|e| format!("Failed to encode payment: {}", e))?;

    // Retry request with payment
    api::make_request(api_url, endpoint, body, Some(payment_header))
        .await
        .map_err(|e| format!("Request with payment failed: {}", e))
}

#[function_component]
fn App() -> Html {
    // Wallet state
    let wallet_account = use_state(|| None::<wallet::WalletAccount>);
    let wallet_initialized = use_state(|| false);
    let wallet_error = use_state(|| None::<String>);

    // API state
    let api_url = use_state(|| "http://127.0.0.1:3000".to_string());
    let endpoints = use_state(|| api::get_endpoints());
    let selected_endpoint = use_state(|| 0usize);
    let request_body = use_state(|| String::new());
    let response = use_state(|| None::<api::ApiResponse>);
    let is_loading = use_state(|| false);
    let active_tab = use_state(|| "body".to_string());

    // Initialize wallet on mount
    {
        let wallet_initialized = wallet_initialized.clone();
        let wallet_account = wallet_account.clone();
        let wallet_error = wallet_error.clone();

        use_effect_with((), move |_| {
            spawn_local(async move {
                match wallet::initialize().await {
                    Ok(_) => {
                        wallet_initialized.set(true);
                        if let Ok(account) = wallet::get_account().await {
                            wallet_account.set(Some(account));
                        }
                    }
                    Err(e) => {
                        wallet_error.set(Some(e));
                    }
                }
            });
            || ()
        });
    }

    // Poll wallet account state
    {
        let wallet_account = wallet_account.clone();

        use_effect_with((), move |_| {
            let interval = gloo_timers::callback::Interval::new(1000, move || {
                let wallet_account = wallet_account.clone();
                spawn_local(async move {
                    if let Ok(account) = wallet::get_account().await {
                        wallet_account.set(Some(account));
                    }
                });
            });

            move || drop(interval)
        });
    }

    // Update request body when endpoint changes
    {
        let endpoints = endpoints.clone();
        let selected_endpoint = selected_endpoint.clone();
        let request_body = request_body.clone();

        use_effect_with(selected_endpoint.clone(), move |idx| {
            if let Some(endpoint) = endpoints.get(**idx) {
                if let Some(default_body) = &endpoint.default_body {
                    request_body.set(default_body.clone());
                } else {
                    request_body.set(String::new());
                }
            }
            || ()
        });
    }

    // Handlers
    let on_connect_wallet = {
        let wallet_error = wallet_error.clone();
        let wallet_account = wallet_account.clone();
        
        Callback::from(move |_| {
            let wallet_error = wallet_error.clone();
            let wallet_account = wallet_account.clone();
            spawn_local(async move {
                match wallet::connect().await {
                    Ok(_) => {
                        wallet_error.set(None);
                        if let Ok(account) = wallet::get_account().await {
                            wallet_account.set(Some(account));
                        }
                    }
                    Err(e) => {
                        wallet_error.set(Some(e));
                    }
                }
            });
        })
    };

    let on_disconnect_wallet = {
        let wallet_account = wallet_account.clone();
        
        Callback::from(move |_| {
            let wallet_account = wallet_account.clone();
            spawn_local(async move {
                let _ = wallet::disconnect().await;
                wallet_account.set(None);
            });
        })
    };

    let on_send_request = {
        let api_url = api_url.clone();
        let endpoints = endpoints.clone();
        let selected_endpoint = selected_endpoint.clone();
        let request_body = request_body.clone();
        let response = response.clone();
        let is_loading = is_loading.clone();
        let wallet_account = wallet_account.clone();

        Callback::from(move |_| {
            let api_url = (*api_url).clone();
            let endpoints = endpoints.clone();
            let selected_endpoint = *selected_endpoint;
            let request_body = (*request_body).clone();
            let response = response.clone();
            let is_loading = is_loading.clone();
            let wallet_account = wallet_account.clone();

            spawn_local(async move {
                if let Some(endpoint) = endpoints.get(selected_endpoint) {
                    is_loading.set(true);
                    
                    let body = if endpoint.method == "POST" && !request_body.is_empty() {
                        Some(request_body.clone())
                    } else {
                        None
                    };

                    // First attempt without payment
                    match api::make_request(&api_url, endpoint, body.clone(), None).await {
                        Ok(resp) => {
                            // Check if payment is required (402)
                            if resp.status == 402 {
                                // Try to handle payment automatically
                                if let Some(account) = (*wallet_account).clone() {
                                    if account.is_connected {
                                        // Parse payment requirements
                                        if let Ok(payment_resp) = serde_json::from_str::<payment::PaymentRequirementsResponse>(&resp.body) {
                                            if let Some(requirements) = payment_resp.accepts.first() {
                                                // Show initial 402 response
                                                response.set(Some(resp.clone()));
                                                
                                                // Attempt payment
                                                match handle_payment(requirements, &account, &api_url, endpoint, body).await {
                                                    Ok(paid_resp) => {
                                                        response.set(Some(paid_resp));
                                                    }
                                                    Err(e) => {
                                                        // Show payment error
                                                        response.set(Some(api::ApiResponse {
                                                            status: 402,
                                                            status_text: "Payment Failed".to_string(),
                                                            headers: vec![],
                                                            body: format!("{{\"error\": \"Payment failed: {}\", \"original_requirements\": {}}}", e, resp.body),
                                                        }));
                                                    }
                                                }
                                            } else {
                                                response.set(Some(resp));
                                            }
                                        } else {
                                            response.set(Some(resp));
                                        }
                                    } else {
                                        // Wallet not connected, show 402
                                        response.set(Some(resp));
                                    }
                                } else {
                                    // No wallet, show 402
                                    response.set(Some(resp));
                                }
                            } else {
                                // Not a payment required response
                                response.set(Some(resp));
                            }
                        }
                        Err(e) => {
                            response.set(Some(api::ApiResponse {
                                status: 0,
                                status_text: "Error".to_string(),
                                headers: vec![],
                                body: format!("{{\"error\": \"{}\"}}", e),
                            }));
                        }
                    }
                    
                    is_loading.set(false);
                }
            });
        })
    };

    html! {
        <div class="container">
            <div class="header">
                <h1>{"🦀 SnapRAG API Tester"}</h1>
                <p>{"Test SnapRAG API endpoints with MetaMask payment integration"}</p>
            </div>

            <div class="main-content">
                // Sidebar
                <div class="sidebar">
                    // Wallet section
                    <div class="wallet-section">
                        <h3>{"💳 Wallet"}</h3>
                        
                        {
                            if !*wallet_initialized {
                                html! {
                                    <div class="wallet-status">
                                        {"Initializing MetaMask..."}
                                    </div>
                                }
                            } else if let Some(error) = (*wallet_error).clone() {
                                html! {
                                    <div class="wallet-status disconnected">
                                        <div><strong>{"⚠️ Error"}</strong></div>
                                        <div style="font-size: 12px; margin-top: 5px;">{error}</div>
                                    </div>
                                }
                            } else if let Some(account) = (*wallet_account).clone() {
                                if account.is_connected {
                                    html! {
                                        <>
                                            <div class="wallet-status connected">
                                                <div><strong>{"✅ Connected"}</strong></div>
                                                <div style="font-size: 12px; margin-top: 5px; font-family: monospace;">
                                                    {format!("{}...{}", 
                                                        account.address.as_ref().map(|a| &a[..6]).unwrap_or(""),
                                                        account.address.as_ref().map(|a| &a[a.len()-4..]).unwrap_or("")
                                                    )}
                                                </div>
                                                {
                                                    if let Some(chain_id) = account.chain_id {
                                                        html! {
                                                            <div style="font-size: 11px; margin-top: 3px; color: #666;">
                                                                {format!("Chain: {}", chain_id)}
                                                            </div>
                                                        }
                                                    } else {
                                                        html! {}
                                                    }
                                                }
                                            </div>
                                            <button onclick={on_disconnect_wallet} class="secondary">
                                                {"Disconnect"}
                                            </button>
                                        </>
                                    }
                                } else {
                                    html! {
                                        <>
                                            <div class="wallet-status disconnected">
                                                <div><strong>{"❌ Not Connected"}</strong></div>
                                            </div>
                                            <button onclick={on_connect_wallet}>
                                                {"Connect MetaMask"}
                                            </button>
                                        </>
                                    }
                                }
                            } else {
                                html! {
                                    <>
                                        <div class="wallet-status disconnected">
                                            <div><strong>{"❌ Not Connected"}</strong></div>
                                        </div>
                                        <button onclick={on_connect_wallet}>
                                            {"Connect MetaMask"}
                                        </button>
                                    </>
                                }
                            }
                        }
                    </div>

                    // API URL section
                    <div class="form-group" style="margin-bottom: 20px;">
                        <label>{"API Base URL"}</label>
                        <input 
                            type="text" 
                            value={(*api_url).clone()}
                            oninput={
                                let api_url = api_url.clone();
                                Callback::from(move |e: InputEvent| {
                                    if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                                        api_url.set(input.value());
                                    }
                                })
                            }
                        />
                    </div>

                    // Endpoints list
                    <h3>{"📡 Endpoints"}</h3>
                    <div class="endpoint-list">
                        {
                            endpoints.iter().enumerate().map(|(idx, endpoint)| {
                                let selected_endpoint = selected_endpoint.clone();
                                let is_active = idx == *selected_endpoint;
                                
                                html! {
                                    <div 
                                        class={classes!("endpoint-item", is_active.then(|| "active"))}
                                        onclick={Callback::from(move |_| {
                                            selected_endpoint.set(idx);
                                        })}
                                    >
                                        <div>
                                            <span class={classes!("endpoint-method", format!("method-{}", endpoint.method.to_lowercase()))}>
                                                {&endpoint.method}
                                            </span>
                                            <span style="font-size: 13px; font-weight: 600;">{&endpoint.name}</span>
                                            <span class={classes!("tier-badge", format!("tier-{}", endpoint.tier.to_lowercase()))}>
                                                {&endpoint.tier}
                                            </span>
                                        </div>
                                        <div style="font-size: 11px; color: #999; margin-top: 4px; font-family: monospace;">
                                            {&endpoint.path}
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                </div>

                // Main content
                <div class="content">
                    {
                        if let Some(endpoint) = endpoints.get(*selected_endpoint) {
                            html! {
                                <>
                                    // Request section
                                    <div class="request-section">
                                        <h3>{format!("{} {}", endpoint.method, endpoint.name)}</h3>
                                        <p style="color: #666; font-size: 14px; margin-bottom: 15px;">
                                            {&endpoint.description}
                                        </p>

                                        {
                                            if endpoint.requires_payment {
                                                html! {
                                                    <div class="alert alert-warning">
                                                        {"⚠️ This endpoint requires payment. "} 
                                                        <span class="code">{format!("{} tier", endpoint.tier)}</span>
                                                        {" - Connect wallet to test with x402 payment."}
                                                    </div>
                                                }
                                            } else {
                                                html! {
                                                    <div class="alert alert-info">
                                                        {"ℹ️ This is a free endpoint - no payment required."}
                                                    </div>
                                                }
                                            }
                                        }

                                        {
                                            if endpoint.method == "POST" {
                                                html! {
                                                    <div class="form-group">
                                                        <label>{"Request Body (JSON)"}</label>
                                                        <textarea
                                                            value={(*request_body).clone()}
                                                            oninput={
                                                                let request_body = request_body.clone();
                                                                Callback::from(move |e: InputEvent| {
                                                                    if let Some(textarea) = e.target_dyn_into::<web_sys::HtmlTextAreaElement>() {
                                                                        request_body.set(textarea.value());
                                                                    }
                                                                })
                                                            }
                                                            placeholder={r#"{"key": "value"}"#}
                                                        />
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }
                                        }

                                        <button 
                                            onclick={on_send_request}
                                            disabled={*is_loading}
                                        >
                                            {
                                                if *is_loading {
                                                    "Sending..."
                                                } else {
                                                    "Send Request"
                                                }
                                            }
                                        </button>
                                    </div>

                                    // Response section
                                    {
                                        if let Some(resp) = (*response).clone() {
                                            html! {
                                                <div class="response-section">
                                                    <div class="response-header">
                                                        <h3>{"Response"}</h3>
                                                        <span class={classes!(
                                                            "response-status",
                                                            if resp.status == 200 {
                                                                "status-success"
                                                            } else if resp.status == 402 {
                                                                "status-payment"
                                                            } else {
                                                                "status-error"
                                                            }
                                                        )}>
                                                            {format!("{} {}", resp.status, resp.status_text)}
                                                        </span>
                                                    </div>

                                                    {
                                                        // Show payment info for 402 responses
                                                        if resp.status == 402 {
                                                            html! {
                                                                <div class="alert alert-warning" style="margin-bottom: 15px;">
                                                                    {
                                                                        if let Some(account) = (*wallet_account).clone() {
                                                                            if account.is_connected {
                                                                                html! { 
                                                                                    <>
                                                                                        {"💳 Payment signature requested. "}
                                                                                        {"Please sign the payment in MetaMask to continue."}
                                                                                    </>
                                                                                }
                                                                            } else {
                                                                                html! {
                                                                                    <>
                                                                                        {"⚠️ Payment required but wallet not connected. "}
                                                                                        {"Please connect your wallet and try again."}
                                                                                    </>
                                                                                }
                                                                            }
                                                                        } else {
                                                                            html! {
                                                                                <>
                                                                                    {"⚠️ Payment required but no wallet connected. "}
                                                                                    {"Please connect MetaMask and try again."}
                                                                                </>
                                                                            }
                                                                        }
                                                                    }
                                                                </div>
                                                            }
                                                        } else {
                                                            html! {}
                                                        }
                                                    }

                                                    <div class="tabs">
                                                        <button 
                                                            class={classes!("tab", (*active_tab == "body").then(|| "active"))}
                                                            onclick={
                                                                let active_tab = active_tab.clone();
                                                                Callback::from(move |_| active_tab.set("body".to_string()))
                                                            }
                                                        >
                                                            {"Body"}
                                                        </button>
                                                        <button 
                                                            class={classes!("tab", (*active_tab == "headers").then(|| "active"))}
                                                            onclick={
                                                                let active_tab = active_tab.clone();
                                                                Callback::from(move |_| active_tab.set("headers".to_string()))
                                                            }
                                                        >
                                                            {format!("Headers ({})", resp.headers.len())}
                                                        </button>
                                                    </div>

                                                    {
                                                        if *active_tab == "body" {
                                                            // Try to pretty-print JSON
                                                            let formatted = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&resp.body) {
                                                                serde_json::to_string_pretty(&json).unwrap_or(resp.body.clone())
                                                            } else {
                                                                resp.body.clone()
                                                            };

                                                            html! {
                                                                <pre class="response-body">{formatted}</pre>
                                                            }
                                                        } else {
                                                            html! {
                                                                <div class="response-body">
                                                                    {
                                                                        resp.headers.iter().map(|(k, v)| {
                                                                            html! {
                                                                                <div style="margin-bottom: 5px;">
                                                                                    <span style="color: #4fc3f7;">{k}{": "}</span>
                                                                                    <span style="color: #ce93d8;">{v}</span>
                                                                                </div>
                                                                            }
                                                                        }).collect::<Html>()
                                                                    }
                                                                </div>
                                                            }
                                                        }
                                                    }
                                                </div>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </>
                            }
                        } else {
                            html! {
                                <div class="loading">{"Select an endpoint to test"}</div>
                            }
                        }
                    }
                </div>
            </div>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}

