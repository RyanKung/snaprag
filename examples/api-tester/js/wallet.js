/**
 * MetaMask Wallet Integration
 * Simple MetaMask connector using window.ethereum
 */

let ethereum = null;
let currentAccount = null;
let currentChainId = null;

/**
 * Initialize MetaMask connection
 */
export async function initWallet() {
    try {
        if (typeof window.ethereum !== 'undefined') {
            ethereum = window.ethereum;
            
            // Setup event listeners
            ethereum.on('accountsChanged', handleAccountsChanged);
            ethereum.on('chainChanged', handleChainChanged);
            ethereum.on('disconnect', handleDisconnect);
            
            // Get initial state
            try {
                const accounts = await ethereum.request({ method: 'eth_accounts' });
                if (accounts.length > 0) {
                    currentAccount = accounts[0];
                }
                
                currentChainId = await ethereum.request({ method: 'eth_chainId' });
            } catch (err) {
                console.warn('Could not get initial state:', err);
            }
            
            return {
                success: true,
                error: null,
                data: null
            };
        } else {
            return {
                success: false,
                error: 'MetaMask is not installed',
                data: null
            };
        }
    } catch (error) {
        return {
            success: false,
            error: error.message,
            data: null
        };
    }
}

/**
 * Get current wallet account info
 */
export async function getWalletAccount() {
    try {
        const account = {
            address: currentAccount,
            is_connected: !!currentAccount,
            is_connecting: false,
            is_disconnected: !currentAccount,
            chain_id: currentChainId ? parseInt(currentChainId, 16) : null,
            connector: 'MetaMask'
        };
        
        return {
            success: true,
            error: null,
            data: JSON.stringify(account)
        };
    } catch (error) {
        return {
            success: false,
            error: error.message,
            data: null
        };
    }
}

/**
 * Connect wallet (request accounts)
 */
export async function connectWallet() {
    try {
        if (!ethereum) {
            throw new Error('MetaMask not initialized');
        }
        
        const accounts = await ethereum.request({ 
            method: 'eth_requestAccounts' 
        });
        
        if (accounts.length > 0) {
            currentAccount = accounts[0];
            currentChainId = await ethereum.request({ method: 'eth_chainId' });
            
            return {
                success: true,
                error: null,
                data: JSON.stringify({
                    address: currentAccount,
                    chain_id: parseInt(currentChainId, 16)
                })
            };
        } else {
            throw new Error('No accounts found');
        }
    } catch (error) {
        return {
            success: false,
            error: error.message,
            data: null
        };
    }
}

/**
 * Disconnect wallet
 */
export async function disconnectWallet() {
    try {
        currentAccount = null;
        return {
            success: true,
            error: null,
            data: null
        };
    } catch (error) {
        return {
            success: false,
            error: error.message,
            data: null
        };
    }
}

/**
 * Switch to specific chain
 */
export async function switchToChain(chainIdHex) {
    try {
        if (!ethereum) {
            throw new Error('MetaMask not initialized');
        }
        
        await ethereum.request({
            method: 'wallet_switchEthereumChain',
            params: [{ chainId: chainIdHex }],
        });
        
        return {
            success: true,
            error: null,
            data: null
        };
    } catch (error) {
        // Chain not added, try to add it
        if (error.code === 4902) {
            return {
                success: false,
                error: 'Chain not added to MetaMask',
                data: null
            };
        }
        
        return {
            success: false,
            error: error.message,
            data: null
        };
    }
}

/**
 * Sign message with wallet
 */
export async function signWalletMessage(message) {
    try {
        if (!ethereum || !currentAccount) {
            throw new Error('Wallet not connected');
        }
        
        const signature = await ethereum.request({
            method: 'personal_sign',
            params: [message, currentAccount],
        });
        
        return {
            success: true,
            error: null,
            data: signature
        };
    } catch (error) {
        return {
            success: false,
            error: error.message,
            data: null
        };
    }
}

/**
 * Sign typed data (EIP-712)
 */
export async function signTypedData(typedData) {
    try {
        if (!ethereum || !currentAccount) {
            throw new Error('Wallet not connected');
        }
        
        const signature = await ethereum.request({
            method: 'eth_signTypedData_v4',
            params: [currentAccount, typedData],
        });
        
        return {
            success: true,
            error: null,
            data: signature
        };
    } catch (error) {
        return {
            success: false,
            error: error.message,
            data: null
        };
    }
}

// Event handlers
function handleAccountsChanged(accounts) {
    if (accounts.length === 0) {
        currentAccount = null;
    } else {
        currentAccount = accounts[0];
    }
    console.log('Account changed:', currentAccount);
}

function handleChainChanged(chainId) {
    currentChainId = chainId;
    console.log('Chain changed:', chainId);
    // Reload page on chain change (recommended by MetaMask)
    window.location.reload();
}

function handleDisconnect() {
    currentAccount = null;
    currentChainId = null;
    console.log('Disconnected');
}

/**
 * Cleanup event listeners
 */
export async function cleanupWallet() {
    try {
        if (ethereum) {
            ethereum.removeListener('accountsChanged', handleAccountsChanged);
            ethereum.removeListener('chainChanged', handleChainChanged);
            ethereum.removeListener('disconnect', handleDisconnect);
        }
        return {
            success: true,
            error: null,
            data: null
        };
    } catch (error) {
        return {
            success: false,
            error: error.message,
            data: null
        };
    }
}

