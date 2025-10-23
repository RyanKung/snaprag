# SnapRAG Scripts Directory

This directory contains specialized SQL scripts for advanced PostgreSQL optimization scenarios.

## ⚠️ Important Notice

**Most optimization tasks are now handled by the integrated `fastsync` command:**

```bash
# Instead of manual scripts, use:
snaprag fastsync enable --profile 346gb --force
snaprag fastsync disable --force
snaprag fastsync status
```

## 📁 Remaining Scripts

### 🚀 Extreme Performance Scripts

#### `unlogged_mode_enable.sql`
- **Purpose**: Convert tables to UNLOGGED for maximum write speed
- **Performance**: +100-300% faster writes (20-45k records/sec)
- **Risk**: ⚠️ Data loss on PostgreSQL crash/power failure
- **Usage**: Only for initial full sync scenarios where data can be re-synced

#### `unlogged_mode_disable.sql`
- **Purpose**: Convert UNLOGGED tables back to LOGGED
- **Usage**: Run after sync completion to restore data safety

### 📚 Configuration Reference

#### `postgresql_turbo_config.sql`
- **Purpose**: Reference PostgreSQL configuration for maximum performance
- **Usage**: Manual configuration reference (mostly replaced by `fastsync`)

### 🔧 Utility Scripts

#### `backfill_links_manual.sh`
- **Purpose**: Manual backfill script for specific high-value users
- **Usage**: Temporary fix for link processing issues

## 🎯 Usage Recommendations

### Normal Operations
Use the integrated `fastsync` command:
```bash
# Enable fast sync mode
snaprag fastsync enable --profile 346gb --force

# Check status
snaprag fastsync status

# Disable after sync
snaprag fastsync disable --force
```

### Extreme Scenarios
Only use the remaining scripts for:
- **UNLOGGED mode**: When you need maximum speed and can afford data loss risk
- **Manual configuration**: When you need to customize beyond `fastsync` profiles
- **Special backfill**: For specific user data recovery

## 📊 Performance Comparison

| Method | Speed Boost | Safety | Complexity |
|--------|-------------|---------|------------|
| `fastsync` | +50-80% | ✅ Safe | 🟢 Simple |
| UNLOGGED | +100-300% | ⚠️ Risky | 🔴 Complex |

## 🚨 Warning

**UNLOGGED mode scripts are for experts only!**
- Data loss risk on crashes
- Requires exclusive table locks
- Must be applied to empty tables
- Cannot be used during active sync

Use `fastsync` for normal operations unless you specifically need the extreme performance of UNLOGGED mode.
