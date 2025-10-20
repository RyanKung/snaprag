#!/bin/bash
# Manual script to backfill links for specific high-value users
# This is a temporary fix until we re-sync with the corrected link processing

set -e

DB_URL="postgresql://snaprag:hackinthebox_24601@192.168.1.192/snaprag"
SNAPCHAIN_URL="${SNAPCHAIN_HTTP_URL:-http://192.168.1.192:2285}"

# Array of high-value FIDs to backfill
FIDS=(99 1 2 3 4 5)  # Jesse Pollak, dwr, v, etc.

echo "🔗 Starting links backfill from Snapchain..."
echo "Database: $DB_URL"
echo "Snapchain: $SNAPCHAIN_URL"
echo ""

total_inserted=0

for fid in "${FIDS[@]}"; do
    echo "Processing FID $fid..."
    
    # Fetch links from Snapchain API
    response=$(curl -s "${SNAPCHAIN_URL}/v1/linksByFid?fid=${fid}&pageSize=1000")
    
    # Parse JSON and extract links (using jq)
    if command -v jq &> /dev/null; then
        # Extract each link message and insert
        echo "$response" | jq -r '.messages[] | 
            @json' | while read -r message; do
            
            # Extract fields from JSON
            target_fid=$(echo "$message" | jq -r '.data.body.link_body.target_fid // 0')
            link_type=$(echo "$message" | jq -r '.data.body.link_body.type // "follow"')
            timestamp=$(echo "$message" | jq -r '.data.timestamp // 0')
            hash=$(echo "$message" | jq -r '.hash' | xxd -r -p | xxd -p -c 256)
            
            if [ "$target_fid" != "0" ] && [ "$target_fid" != "null" ]; then
                # Insert into database
                psql "$DB_URL" -c "
                    INSERT INTO links (fid, target_fid, link_type, timestamp, message_hash)
                    VALUES ($fid, $target_fid, '$link_type', $timestamp, decode('$hash', 'hex'))
                    ON CONFLICT (message_hash) DO NOTHING
                " > /dev/null 2>&1
                
                ((total_inserted++))
            fi
        done
        
        echo "  ✅ Processed FID $fid"
    else
        echo "  ⚠️  jq not found, skipping JSON parsing"
    fi
done

echo ""
echo "✅ Backfill complete! Inserted ~$total_inserted links"
echo ""
echo "Verify with: psql $DB_URL -c \"SELECT COUNT(*) FROM links;\""

