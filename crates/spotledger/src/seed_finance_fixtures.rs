//! `spotledger seed-finance-fixtures` — Phase 8 baseline fixture seeding.
//!
//! Current scope (safe, idempotent):
//! - Currencies: PKR (base), USD, AED
//! - Default Chart of Accounts: configurable name (default: SpotLedger Standard PKR)
//! - Account Groups: Section 2 category/range structure
//! - Default Accounts: baseline 4-digit chart
//! - Financial Statement Versions:
//!   - Pakistan Statutory (purpose: Statutory)
//!   - Management Reporting (purpose: Management)

use anyhow::{Context, Result};
use serde_json::json;
use spotledger_core::config::SiteConfig;
use spotledger_db::connection::connect;
use spotledger_db::document::upsert_doc;

use crate::cli::SeedFinanceFixturesArgs;

pub async fn seed_finance_fixtures(args: SeedFinanceFixturesArgs) -> Result<()> {
    let bench = args
        .bench
        .canonicalize()
        .unwrap_or_else(|_| args.bench.clone());

    let config_path = bench
        .join("sites")
        .join(&args.site)
        .join("site_config.toml");
    let config_str = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Reading {}", config_path.display()))?;
    let cfg: SiteConfig =
        toml::from_str(&config_str).context("Parsing site_config.toml")?;

    println!(
        "Connecting to {} (ns={}, db={}) ...",
        cfg.database.url, cfg.database.ns, cfg.database.db
    );
    let db = connect(&cfg.database)
        .await
        .context("Connecting to SurrealDB")?;
    println!("Connected.");

    // 1) Currency fixtures
    seed_currency(
        &db,
        "PKR",
        "PKR",
        "Rs",
        "Paisa",
        100,
        0.01,
        true,
    )
    .await?;
    seed_currency(
        &db,
        "USD",
        "USD",
        "$",
        "Cent",
        100,
        0.01,
        false,
    )
    .await?;
    seed_currency(
        &db,
        "AED",
        "AED",
        "AED",
        "Fils",
        100,
        0.01,
        false,
    )
    .await?;

    // 2) Default Chart of Accounts
    let coa_name = args.coa_name.trim().to_string();
    if coa_name.is_empty() {
        anyhow::bail!("--coa-name must not be empty");
    }

    upsert_doc(
        &db,
        "Chart of Accounts",
        &coa_name,
        &json!({
            "name": coa_name,
            "coa_name": args.coa_name,
            "currency": "PKR",
            "owner": "Administrator",
            "modified_by": "Administrator",
            "description": "Default Chart of Accounts for Phase 8 baseline fixtures"
        }),
    )
    .await
    .context("Seeding Chart of Accounts fixture")?;

    // 3) Account Group fixtures (Section 2 category/range convention)
    let groups = [
        ("Fixed Asset", "Fixed Asset", "1000", "1499"),
        ("Current Asset", "Current Asset", "1500", "1999"),
        ("Cash and Bank", "Cash and Bank", "2000", "2199"),
        ("Trade Receivable", "Trade Receivable", "2200", "2299"),
        ("Inventory", "Inventory", "2300", "2499"),
        ("Long Term Liability", "Long Term Liability", "2500", "2699"),
        ("Trade Payable", "Trade Payable", "2700", "2899"),
        ("Current Liability", "Current Liability", "2900", "2999"),
        ("Equity", "Equity", "3000", "3999"),
        ("Revenue", "Revenue", "4000", "4999"),
        ("Cost of Sales", "Cost of Sales", "5000", "5999"),
        ("Expense", "Expense", "6000", "6999"),
        ("Tax Account", "Tax Account", "7000", "7099"),
    ];
    for (group_name, account_category, number_from, number_to) in groups {
        seed_account_group(
            &db,
            group_name,
            &coa_name,
            account_category,
            number_from,
            number_to,
        )
        .await?;
    }

    // 4) Default account fixtures (baseline 4-digit chart)
    let accounts = [
        ("1000", "Land and Building", "Fixed Asset"),
        ("1100", "Plant and Machinery", "Fixed Asset"),
        ("1200", "Furniture and Fixtures", "Fixed Asset"),
        ("1500", "Inventory Asset", "Current Asset"),
        ("1600", "Prepaid Expenses", "Current Asset"),
        ("2000", "Cash in Hand", "Cash and Bank"),
        ("2100", "Bank Current Account", "Cash and Bank"),
        ("2200", "Trade Debtors", "Trade Receivable"),
        ("2300", "Stock in Trade", "Inventory"),
        ("2500", "Long Term Finance", "Long Term Liability"),
        ("2700", "Trade Creditors", "Trade Payable"),
        ("2800", "One-Time Payable", "Trade Payable"),
        ("2900", "Accrued Expenses", "Current Liability"),
        ("2950", "Tax Payable", "Tax Account"),
        ("3000", "Paid-up Capital", "Equity"),
        ("3200", "Retained Earnings", "Equity"),
        ("4000", "Sales Revenue", "Revenue"),
        ("4100", "Service Revenue", "Revenue"),
        ("5000", "Cost of Goods Sold", "Cost of Sales"),
        ("5100", "Direct Wages", "Cost of Sales"),
        ("6000", "Salaries Expense", "Expense"),
        ("6100", "Rent Expense", "Expense"),
        ("6200", "Utilities Expense", "Expense"),
        ("6300", "Depreciation Expense", "Expense"),
        ("6400", "Bank Charges", "Expense"),
    ];
    for (account_number, account_name, account_group) in accounts {
        seed_account(
            &db,
            account_number,
            account_name,
            &coa_name,
            account_group,
        )
        .await?;
    }

    // 5) Baseline Financial Statement Versions
    upsert_doc(
        &db,
        "Financial Statement Version",
        "Pakistan Statutory",
        &json!({
            "name": "Pakistan Statutory",
            "fsv_name": "Pakistan Statutory",
            "chart_of_accounts": args.coa_name,
            "purpose": "Statutory",
            "owner": "Administrator",
            "modified_by": "Administrator",
            "description": "Default statutory statement layout"
        }),
    )
    .await
    .context("Seeding FSV: Pakistan Statutory")?;

    upsert_doc(
        &db,
        "Financial Statement Version",
        "Management Reporting",
        &json!({
            "name": "Management Reporting",
            "fsv_name": "Management Reporting",
            "chart_of_accounts": args.coa_name,
            "purpose": "Management",
            "owner": "Administrator",
            "modified_by": "Administrator",
            "description": "Default management reporting layout"
        }),
    )
    .await
    .context("Seeding FSV: Management Reporting")?;

    println!("Phase 8 fixtures seeded successfully.");
    println!("- Currency: PKR (base), USD, AED");
    println!("- Chart of Accounts: {}", args.coa_name);
    println!("- Account Groups: {}", groups.len());
    println!("- Default Accounts: {}", accounts.len());
    println!("- FSVs: Pakistan Statutory, Management Reporting");

    Ok(())
}

async fn seed_currency(
    db: &spotledger_db::DbAdapter,
    name: &str,
    currency_name: &str,
    symbol: &str,
    fraction: &str,
    fraction_units: i64,
    smallest_currency_fraction_value: f64,
    is_base: bool,
) -> Result<()> {
    let is_base_int = if is_base { 1 } else { 0 };

    upsert_doc(
        db,
        "Currency",
        name,
        &json!({
            "name": name,
            "currency_name": currency_name,
            "symbol": symbol,
            "fraction": fraction,
            "fraction_units": fraction_units,
            "smallest_currency_fraction_value": smallest_currency_fraction_value,
            "enabled": 1,
            "is_base": is_base_int,
            "owner": "Administrator",
            "modified_by": "Administrator"
        }),
    )
    .await
    .with_context(|| format!("Seeding Currency fixture: {name}"))?;

    Ok(())
}

async fn seed_account_group(
    db: &spotledger_db::DbAdapter,
    group_name: &str,
    chart_of_accounts: &str,
    account_category: &str,
    account_number_from: &str,
    account_number_to: &str,
) -> Result<()> {
    upsert_doc(
        db,
        "Account Group",
        group_name,
        &json!({
            "name": group_name,
            "group_name": group_name,
            "chart_of_accounts": chart_of_accounts,
            "account_category": account_category,
            "account_number_from": account_number_from,
            "account_number_to": account_number_to,
            "party_required": 0,
            "owner": "Administrator",
            "modified_by": "Administrator",
            "description": "Seeded by phase 8 finance fixtures"
        }),
    )
    .await
    .with_context(|| format!("Seeding Account Group fixture: {group_name}"))?;

    Ok(())
}

async fn seed_account(
    db: &spotledger_db::DbAdapter,
    account_number: &str,
    account_name: &str,
    chart_of_accounts: &str,
    account_group: &str,
) -> Result<()> {
    upsert_doc(
        db,
        "Account",
        account_number,
        &json!({
            "name": account_number,
            "account_number": account_number,
            "account_name": account_name,
            "chart_of_accounts": chart_of_accounts,
            "account_group": account_group,
            "is_group": 0,
            "owner": "Administrator",
            "modified_by": "Administrator",
            "description": "Seeded by phase 8 finance fixtures"
        }),
    )
    .await
    .with_context(|| format!("Seeding Account fixture: {account_number}"))?;

    Ok(())
}
