# Phase 3: Financial & Administrative Features Implementation

## Overview

Phase 3 introduces comprehensive financial management and administrative automation capabilities. This phase transforms the Personal AI Assistant into a financial advisor and administrative assistant, handling banking integration, expense tracking, investment monitoring, bill management, and tax preparation assistance.

## Feature Scope

### Core Financial Features
1. **Banking Integration**: Secure connection to bank accounts and credit cards
2. **Expense Tracking**: Automatic categorization and analysis of transactions
3. **Budget Management**: Intelligent budgeting with AI-powered insights
4. **Investment Monitoring**: Portfolio tracking and performance analysis
5. **Bill Management**: Automated bill detection, reminders, and payment scheduling
6. **Tax Preparation**: Document organization and tax-related insights
7. **Financial Reporting**: Comprehensive financial dashboards and reports

### Administrative Features
1. **Document Management**: Intelligent filing and categorization
2. **Insurance Tracking**: Policy management and renewal reminders
3. **Subscription Management**: Recurring payment tracking and optimization
4. **Contract Management**: Important date tracking and renewal alerts
5. **Compliance Monitoring**: Regulatory deadlines and requirements

## Architecture Implementation

### 1. Financial Service Layer

```toml
# Add to workspace Cargo.toml
[workspace]
members = [
    # ... existing crates
    "crates/financial",
    "crates/banking",
    "crates/admin"
]

# crates/financial/Cargo.toml
[package]
name = "financial"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
uuid = { workspace = true }
reqwest = { version = "0.11", features = ["json"] }
rust_decimal = { version = "1.33", features = ["serde"] }
regex = "1.10"
csv = "1.3"
plaid = "1.0"  # Plaid API for banking integration
alpaca = "0.6"  # Alpaca API for investment data
```

### 2. Core Financial Models

#### Financial Types (`crates/financial/src/models.rs`)

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDate};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub external_id: String,
    pub account_type: AccountType,
    pub institution: Institution,
    pub name: String,
    pub official_name: Option<String>,
    pub balance: Decimal,
    pub available_balance: Option<Decimal>,
    pub currency: String,
    pub is_active: bool,
    pub last_synced: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountType {
    Checking,
    Savings,
    CreditCard,
    Investment,
    Loan,
    Mortgage,
    LineOfCredit,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Institution {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub logo: Option<String>,
    pub primary_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub external_id: String,
    pub account_id: Uuid,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub authorized_date: Option<NaiveDate>,
    pub name: String,
    pub merchant_name: Option<String>,
    pub category: Vec<String>,
    pub subcategory: Option<String>,
    pub transaction_type: TransactionType,
    pub pending: bool,
    pub location: Option<TransactionLocation>,
    pub payment_meta: Option<PaymentMeta>,
    pub custom_category: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Digital,
    Place,
    SpecialMerchant,
    Unresolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLocation {
    pub address: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMeta {
    pub reference_number: Option<String>,
    pub ppd_id: Option<String>,
    pub payee: Option<String>,
    pub by_order_of: Option<String>,
    pub payer: Option<String>,
    pub payment_method: Option<String>,
    pub payment_processor: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub period: BudgetPeriod,
    pub categories: Vec<BudgetCategory>,
    pub total_budget: Decimal,
    pub total_spent: Decimal,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetPeriod {
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    Custom { days: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCategory {
    pub name: String,
    pub allocated_amount: Decimal,
    pub spent_amount: Decimal,
    pub rollover_enabled: bool,
    pub alert_threshold: Option<Decimal>, // Alert when spending exceeds this percentage
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Investment {
    pub id: Uuid,
    pub account_id: Uuid,
    pub symbol: String,
    pub name: String,
    pub quantity: Decimal,
    pub price: Decimal,
    pub value: Decimal,
    pub cost_basis: Decimal,
    pub gain_loss: Decimal,
    pub gain_loss_percent: Decimal,
    pub asset_type: AssetType,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Stock,
    Bond,
    ETF,
    MutualFund,
    Cryptocurrency,
    Cash,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bill {
    pub id: Uuid,
    pub name: String,
    pub vendor: String,
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub frequency: BillFrequency,
    pub category: String,
    pub account_id: Option<Uuid>, // Account to pay from
    pub autopay_enabled: bool,
    pub last_paid_date: Option<NaiveDate>,
    pub last_paid_amount: Option<Decimal>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BillFrequency {
    Weekly,
    BiWeekly,
    Monthly,
    Quarterly,
    SemiAnnually,
    Annually,
    OneTime,
}
```

### 3. Banking Integration Service

#### Banking Service (`crates/banking/src/plaid_service.rs`)

```rust
use anyhow::Result;
use plaid::PlaidClient;
use std::collections::HashMap;

pub struct BankingService {
    plaid_client: PlaidClient,
    storage: Arc<FinancialStorage>,
    ai_service: Arc<AIService>,
    encryption_service: Arc<EncryptionService>,
}

impl BankingService {
    pub async fn new(
        plaid_client_id: &str,
        plaid_secret: &str,
        environment: PlaidEnvironment,
        storage: Arc<FinancialStorage>,
        ai_service: Arc<AIService>,
        encryption_service: Arc<EncryptionService>,
    ) -> Result<Self> {
        let plaid_client = PlaidClient::new(
            plaid_client_id,
            plaid_secret,
            environment,
        );
        
        Ok(Self {
            plaid_client,
            storage,
            ai_service,
            encryption_service,
        })
    }
    
    pub async fn link_account(&self, public_token: &str, user_id: Uuid) -> Result<Vec<Account>> {
        // Exchange public token for access token
        let exchange_response = self.plaid_client
            .item_public_token_exchange(public_token)
            .await?;
        
        let access_token = exchange_response.access_token;
        
        // Encrypt and store access token
        let encrypted_token = self.encryption_service
            .encrypt(&access_token)?;
        self.storage.store_access_token(user_id, encrypted_token).await?;
        
        // Get account information
        let accounts_response = self.plaid_client
            .accounts_get(&access_token)
            .await?;
        
        let mut created_accounts = Vec::new();
        
        for plaid_account in accounts_response.accounts {
            let account = Account {
                id: Uuid::new_v4(),
                external_id: plaid_account.account_id,
                account_type: self.map_account_type(&plaid_account.account_type),
                institution: Institution {
                    id: accounts_response.item.institution_id.clone().unwrap_or_default(),
                    name: "Unknown".to_string(), // This would be fetched separately
                    url: None,
                    logo: None,
                    primary_color: None,
                },
                name: plaid_account.name,
                official_name: plaid_account.official_name,
                balance: plaid_account.balances.current.into(),
                available_balance: plaid_account.balances.available.map(Into::into),
                currency: plaid_account.balances.iso_currency_code.unwrap_or("USD".to_string()),
                is_active: true,
                last_synced: Utc::now(),
                created_at: Utc::now(),
            };
            
            self.storage.store_account(&account).await?;
            created_accounts.push(account);
        }
        
        Ok(created_accounts)
    }
    
    pub async fn sync_transactions(&self, user_id: Uuid, days_back: u32) -> Result<Vec<Transaction>> {
        let accounts = self.storage.get_user_accounts(user_id).await?;
        let mut all_transactions = Vec::new();
        
        for account in accounts {
            let access_token = self.get_decrypted_access_token(user_id).await?;
            
            let start_date = Utc::now().date_naive() - chrono::Duration::days(days_back as i64);
            let end_date = Utc::now().date_naive();
            
            let transactions_response = self.plaid_client
                .transactions_get(
                    &access_token,
                    start_date,
                    end_date,
                    Some(&[account.external_id.clone()]),
                )
                .await?;
            
            for plaid_transaction in transactions_response.transactions {
                // Check if transaction already exists
                if self.storage.transaction_exists(&plaid_transaction.transaction_id).await? {
                    continue;
                }
                
                let transaction = Transaction {
                    id: Uuid::new_v4(),
                    external_id: plaid_transaction.transaction_id,
                    account_id: account.id,
                    amount: plaid_transaction.amount.into(),
                    date: plaid_transaction.date,
                    authorized_date: plaid_transaction.authorized_date,
                    name: plaid_transaction.name,
                    merchant_name: plaid_transaction.merchant_name,
                    category: plaid_transaction.category.unwrap_or_default(),
                    subcategory: None,
                    transaction_type: TransactionType::Place, // Default
                    pending: plaid_transaction.pending,
                    location: plaid_transaction.location.map(|loc| TransactionLocation {
                        address: loc.address,
                        city: loc.city,
                        region: loc.region,
                        postal_code: loc.postal_code,
                        country: loc.country,
                        latitude: loc.lat,
                        longitude: loc.lon,
                    }),
                    payment_meta: None, // Would be mapped from Plaid data
                    custom_category: None,
                    notes: None,
                    tags: Vec::new(),
                    created_at: Utc::now(),
                };
                
                // AI-powered transaction categorization
                let enhanced_transaction = self.enhance_transaction_with_ai(transaction).await?;
                
                self.storage.store_transaction(&enhanced_transaction).await?;
                all_transactions.push(enhanced_transaction);
            }
        }
        
        Ok(all_transactions)
    }
    
    async fn enhance_transaction_with_ai(&self, mut transaction: Transaction) -> Result<Transaction> {
        // Use AI to improve categorization
        let ai_analysis = self.ai_service.analyze_transaction(
            &transaction.name,
            &transaction.merchant_name.unwrap_or_default(),
            &transaction.category,
            transaction.amount,
        ).await?;
        
        transaction.custom_category = ai_analysis.suggested_category;
        transaction.subcategory = ai_analysis.subcategory;
        transaction.tags = ai_analysis.tags;
        
        Ok(transaction)
    }
    
    pub async fn get_account_balances(&self, user_id: Uuid) -> Result<HashMap<Uuid, Decimal>> {
        let accounts = self.storage.get_user_accounts(user_id).await?;
        let access_token = self.get_decrypted_access_token(user_id).await?;
        
        let accounts_response = self.plaid_client
            .accounts_get(&access_token)
            .await?;
        
        let mut balances = HashMap::new();
        
        for plaid_account in accounts_response.accounts {
            if let Some(account) = accounts.iter().find(|a| a.external_id == plaid_account.account_id) {
                balances.insert(account.id, plaid_account.balances.current.into());
            }
        }
        
        Ok(balances)
    }
    
    async fn get_decrypted_access_token(&self, user_id: Uuid) -> Result<String> {
        let encrypted_token = self.storage.get_access_token(user_id).await?;
        self.encryption_service.decrypt(&encrypted_token)
    }
    
    fn map_account_type(&self, plaid_type: &str) -> AccountType {
        match plaid_type {
            "depository" => AccountType::Checking, // Could be savings too
            "credit" => AccountType::CreditCard,
            "investment" => AccountType::Investment,
            "loan" => AccountType::Loan,
            _ => AccountType::Other(plaid_type.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionAnalysis {
    pub suggested_category: Option<String>,
    pub subcategory: Option<String>,
    pub tags: Vec<String>,
    pub is_recurring: bool,
    pub vendor_confidence: f32,
}
```

### 4. Budget Management System

#### Budget Service (`crates/financial/src/budget_service.rs`)

```rust
use anyhow::Result;
use chrono::{NaiveDate, Datelike};
use rust_decimal::Decimal;
use std::collections::HashMap;

pub struct BudgetService {
    storage: Arc<FinancialStorage>,
    ai_service: Arc<AIService>,
    notification_service: Arc<NotificationService>,
}

impl BudgetService {
    pub async fn create_budget_from_spending_history(
        &self,
        user_id: Uuid,
        period: BudgetPeriod,
        analysis_months: u32,
    ) -> Result<Budget> {
        // Analyze historical spending patterns
        let end_date = Utc::now().date_naive();
        let start_date = end_date - chrono::Duration::days((analysis_months * 30) as i64);
        
        let transactions = self.storage
            .get_transactions_by_date_range(user_id, start_date, end_date)
            .await?;
        
        // Group transactions by category and calculate averages
        let mut category_spending: HashMap<String, Vec<Decimal>> = HashMap::new();
        
        for transaction in transactions {
            if transaction.amount > Decimal::ZERO {
                continue; // Skip income transactions
            }
            
            let category = transaction.custom_category
                .clone()
                .or_else(|| transaction.category.first().cloned())
                .unwrap_or_else(|| "Other".to_string());
            
            category_spending
                .entry(category)
                .or_default()
                .push(transaction.amount.abs());
        }
        
        // Calculate monthly averages and create budget categories
        let mut budget_categories = Vec::new();
        let mut total_budget = Decimal::ZERO;
        
        for (category, amounts) in category_spending {
            let monthly_average = amounts.iter().sum::<Decimal>() / Decimal::from(analysis_months);
            let suggested_budget = monthly_average * Decimal::from_f32(1.1).unwrap(); // 10% buffer
            
            budget_categories.push(BudgetCategory {
                name: category,
                allocated_amount: suggested_budget,
                spent_amount: Decimal::ZERO,
                rollover_enabled: false,
                alert_threshold: Some(Decimal::from_f32(0.8).unwrap()), // 80% threshold
            });
            
            total_budget += suggested_budget;
        }
        
        let (start_date, end_date) = self.calculate_budget_period_dates(&period, Utc::now().date_naive());
        
        let budget = Budget {
            id: Uuid::new_v4(),
            name: format!("{:?} Budget", period),
            period,
            categories: budget_categories,
            total_budget,
            total_spent: Decimal::ZERO,
            start_date,
            end_date,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        self.storage.store_budget(&budget).await?;
        Ok(budget)
    }
    
    pub async fn track_spending_against_budget(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
    ) -> Result<BudgetReport> {
        let budget = self.storage.get_budget(budget_id).await?
            .ok_or_else(|| anyhow::anyhow!("Budget not found"))?;
        
        let transactions = self.storage
            .get_transactions_by_date_range(user_id, budget.start_date, budget.end_date)
            .await?;
        
        // Calculate spending by category
        let mut category_spending: HashMap<String, Decimal> = HashMap::new();
        let mut total_spent = Decimal::ZERO;
        
        for transaction in transactions {
            if transaction.amount > Decimal::ZERO {
                continue; // Skip income
            }
            
            let category = transaction.custom_category
                .clone()
                .or_else(|| transaction.category.first().cloned())
                .unwrap_or_else(|| "Other".to_string());
            
            let amount = transaction.amount.abs();
            *category_spending.entry(category).or_default() += amount;
            total_spent += amount;
        }
        
        // Create budget status for each category
        let mut category_statuses = Vec::new();
        let mut alerts = Vec::new();
        
        for category in &budget.categories {
            let spent = category_spending.get(&category.name).cloned().unwrap_or_default();
            let remaining = category.allocated_amount - spent;
            let utilization = if category.allocated_amount > Decimal::ZERO {
                spent / category.allocated_amount
            } else {
                Decimal::ZERO
            };
            
            // Check for alerts
            if let Some(threshold) = category.alert_threshold {
                if utilization >= threshold {
                    alerts.push(BudgetAlert {
                        category: category.name.clone(),
                        message: format!(
                            "Budget alert: {} spending is at {:.1}% of allocated amount",
                            category.name,
                            utilization * Decimal::from(100)
                        ),
                        severity: if utilization >= Decimal::ONE {
                            AlertSeverity::Critical
                        } else {
                            AlertSeverity::Warning
                        },
                    });
                }
            }
            
            category_statuses.push(BudgetCategoryStatus {
                name: category.name.clone(),
                allocated: category.allocated_amount,
                spent,
                remaining,
                utilization,
                is_over_budget: spent > category.allocated_amount,
            });
        }
        
        // Send notifications for alerts
        for alert in &alerts {
            self.notification_service.send_budget_alert(user_id, alert).await?;
        }
        
        Ok(BudgetReport {
            budget_id,
            period_start: budget.start_date,
            period_end: budget.end_date,
            total_allocated: budget.total_budget,
            total_spent,
            total_remaining: budget.total_budget - total_spent,
            overall_utilization: if budget.total_budget > Decimal::ZERO {
                total_spent / budget.total_budget
            } else {
                Decimal::ZERO
            },
            category_statuses,
            alerts,
            generated_at: Utc::now(),
        })
    }
    
    pub async fn suggest_budget_optimizations(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
    ) -> Result<Vec<BudgetOptimization>> {
        let report = self.track_spending_against_budget(user_id, budget_id).await?;
        let mut optimizations = Vec::new();
        
        // Analyze spending patterns
        for status in &report.category_statuses {
            if status.is_over_budget {
                optimizations.push(BudgetOptimization {
                    category: status.name.clone(),
                    optimization_type: OptimizationType::ReduceSpending,
                    description: format!(
                        "Consider reducing spending in {} by {}",
                        status.name,
                        status.spent - status.allocated
                    ),
                    potential_savings: status.spent - status.allocated,
                    priority: OptimizationPriority::High,
                });
            } else if status.utilization < Decimal::from_f32(0.5).unwrap() {
                // Under-utilized categories
                optimizations.push(BudgetOptimization {
                    category: status.name.clone(),
                    optimization_type: OptimizationType::ReallocateFunds,
                    description: format!(
                        "Consider reallocating {} from under-utilized {} category",
                        status.remaining,
                        status.name
                    ),
                    potential_savings: status.remaining,
                    priority: OptimizationPriority::Medium,
                });
            }
        }
        
        // AI-powered optimization suggestions
        let ai_suggestions = self.ai_service
            .suggest_budget_optimizations(&report)
            .await?;
        
        optimizations.extend(ai_suggestions);
        
        Ok(optimizations)
    }
    
    fn calculate_budget_period_dates(&self, period: &BudgetPeriod, reference_date: NaiveDate) -> (NaiveDate, NaiveDate) {
        match period {
            BudgetPeriod::Weekly => {
                let days_since_monday = reference_date.weekday().num_days_from_monday();
                let start = reference_date - chrono::Duration::days(days_since_monday as i64);
                let end = start + chrono::Duration::days(6);
                (start, end)
            }
            BudgetPeriod::Monthly => {
                let start = NaiveDate::from_ymd_opt(reference_date.year(), reference_date.month(), 1).unwrap();
                let end = if reference_date.month() == 12 {
                    NaiveDate::from_ymd_opt(reference_date.year() + 1, 1, 1).unwrap() - chrono::Duration::days(1)
                } else {
                    NaiveDate::from_ymd_opt(reference_date.year(), reference_date.month() + 1, 1).unwrap() - chrono::Duration::days(1)
                };
                (start, end)
            }
            BudgetPeriod::Quarterly => {
                let quarter_start_month = ((reference_date.month() - 1) / 3) * 3 + 1;
                let start = NaiveDate::from_ymd_opt(reference_date.year(), quarter_start_month, 1).unwrap();
                let end = start + chrono::Duration::days(89); // Approximately 3 months
                (start, end)
            }
            BudgetPeriod::Yearly => {
                let start = NaiveDate::from_ymd_opt(reference_date.year(), 1, 1).unwrap();
                let end = NaiveDate::from_ymd_opt(reference_date.year(), 12, 31).unwrap();
                (start, end)
            }
            BudgetPeriod::Custom { days } => {
                let start = reference_date;
                let end = start + chrono::Duration::days(*days as i64 - 1);
                (start, end)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetReport {
    pub budget_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_allocated: Decimal,
    pub total_spent: Decimal,
    pub total_remaining: Decimal,
    pub overall_utilization: Decimal,
    pub category_statuses: Vec<BudgetCategoryStatus>,
    pub alerts: Vec<BudgetAlert>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCategoryStatus {
    pub name: String,
    pub allocated: Decimal,
    pub spent: Decimal,
    pub remaining: Decimal,
    pub utilization: Decimal,
    pub is_over_budget: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    pub category: String,
    pub message: String,
    pub severity: AlertSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetOptimization {
    pub category: String,
    pub optimization_type: OptimizationType,
    pub description: String,
    pub potential_savings: Decimal,
    pub priority: OptimizationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    ReduceSpending,
    ReallocateFunds,
    IncreaseIncome,
    ConsolidateCategories,
    AutomatePayments,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationPriority {
    Low,
    Medium,
    High,
    Critical,
}
```

### 5. Investment Monitoring

#### Investment Service (`crates/financial/src/investment_service.rs`)

```rust
use anyhow::Result;
use alpaca::Alpaca;
use std::collections::HashMap;

pub struct InvestmentService {
    alpaca_client: Alpaca,
    storage: Arc<FinancialStorage>,
    ai_service: Arc<AIService>,
}

impl InvestmentService {
    pub async fn sync_portfolio(&self, user_id: Uuid) -> Result<Vec<Investment>> {
        let accounts = self.storage.get_investment_accounts(user_id).await?;
        let mut all_investments = Vec::new();
        
        for account in accounts {
            // This would integrate with various brokerage APIs
            let positions = self.fetch_positions_from_broker(&account).await?;
            
            for position in positions {
                let current_price = self.get_current_price(&position.symbol).await?;
                let market_value = position.quantity * current_price;
                
                let investment = Investment {
                    id: Uuid::new_v4(),
                    account_id: account.id,
                    symbol: position.symbol,
                    name: position.name,
                    quantity: position.quantity,
                    price: current_price,
                    value: market_value,
                    cost_basis: position.cost_basis,
                    gain_loss: market_value - position.cost_basis,
                    gain_loss_percent: if position.cost_basis > Decimal::ZERO {
                        ((market_value - position.cost_basis) / position.cost_basis) * Decimal::from(100)
                    } else {
                        Decimal::ZERO
                    },
                    asset_type: self.determine_asset_type(&position.symbol).await?,
                    last_updated: Utc::now(),
                };
                
                self.storage.store_investment(&investment).await?;
                all_investments.push(investment);
            }
        }
        
        Ok(all_investments)
    }
    
    pub async fn generate_portfolio_analysis(&self, user_id: Uuid) -> Result<PortfolioAnalysis> {
        let investments = self.storage.get_user_investments(user_id).await?;
        
        let total_value = investments.iter()
            .map(|i| i.value)
            .sum::<Decimal>();
        
        let total_cost_basis = investments.iter()
            .map(|i| i.cost_basis)
            .sum::<Decimal>();
        
        let total_gain_loss = total_value - total_cost_basis;
        let total_gain_loss_percent = if total_cost_basis > Decimal::ZERO {
            (total_gain_loss / total_cost_basis) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };
        
        // Calculate asset allocation
        let mut allocation_by_type: HashMap<AssetType, Decimal> = HashMap::new();
        for investment in &investments {
            *allocation_by_type.entry(investment.asset_type.clone()).or_default() += investment.value;
        }
        
        let allocations: Vec<_> = allocation_by_type.into_iter()
            .map(|(asset_type, value)| AssetAllocation {
                asset_type,
                value,
                percentage: if total_value > Decimal::ZERO {
                    (value / total_value) * Decimal::from(100)
                } else {
                    Decimal::ZERO
                },
            })
            .collect();
        
        // AI-powered analysis
        let ai_insights = self.ai_service
            .analyze_portfolio(&investments, &allocations)
            .await?;
        
        Ok(PortfolioAnalysis {
            total_value,
            total_cost_basis,
            total_gain_loss,
            total_gain_loss_percent,
            asset_allocations: allocations,
            performance_metrics: self.calculate_performance_metrics(&investments).await?,
            risk_metrics: self.calculate_risk_metrics(&investments).await?,
            rebalancing_suggestions: ai_insights.rebalancing_suggestions,
            diversification_score: ai_insights.diversification_score,
            risk_score: ai_insights.risk_score,
            generated_at: Utc::now(),
        })
    }
    
    async fn get_current_price(&self, symbol: &str) -> Result<Decimal> {
        // Use Alpaca or other market data provider
        let quote = self.alpaca_client.get_latest_quote(symbol).await?;
        Ok(quote.bid_price.into())
    }
    
    async fn calculate_performance_metrics(&self, investments: &[Investment]) -> Result<PerformanceMetrics> {
        // Calculate various performance metrics
        let mut metrics = PerformanceMetrics::default();
        
        // This would implement more sophisticated calculations
        // like Sharpe ratio, beta, etc.
        
        Ok(metrics)
    }
    
    async fn calculate_risk_metrics(&self, investments: &[Investment]) -> Result<RiskMetrics> {
        // Calculate portfolio risk metrics
        let mut metrics = RiskMetrics::default();
        
        // This would implement VaR, standard deviation, etc.
        
        Ok(metrics)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioAnalysis {
    pub total_value: Decimal,
    pub total_cost_basis: Decimal,
    pub total_gain_loss: Decimal,
    pub total_gain_loss_percent: Decimal,
    pub asset_allocations: Vec<AssetAllocation>,
    pub performance_metrics: PerformanceMetrics,
    pub risk_metrics: RiskMetrics,
    pub rebalancing_suggestions: Vec<RebalancingSuggestion>,
    pub diversification_score: f32,
    pub risk_score: f32,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAllocation {
    pub asset_type: AssetType,
    pub value: Decimal,
    pub percentage: Decimal,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub annualized_return: Decimal,
    pub sharpe_ratio: f32,
    pub beta: f32,
    pub alpha: f32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub volatility: f32,
    pub value_at_risk: Decimal,
    pub max_drawdown: Decimal,
    pub correlation_matrix: HashMap<String, HashMap<String, f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancingSuggestion {
    pub symbol: String,
    pub current_allocation: Decimal,
    pub target_allocation: Decimal,
    pub action: RebalanceAction,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RebalanceAction {
    Buy(Decimal),
    Sell(Decimal),
    Hold,
}
```

### 6. Bill Management System

#### Bill Service (`crates/financial/src/bill_service.rs`)

```rust
pub struct BillService {
    storage: Arc<FinancialStorage>,
    ai_service: Arc<AIService>,
    notification_service: Arc<NotificationService>,
    payment_service: Arc<PaymentService>,
}

impl BillService {
    pub async fn detect_bills_from_transactions(&self, user_id: Uuid) -> Result<Vec<Bill>> {
        let transactions = self.storage.get_recent_transactions(user_id, 90).await?;
        let mut detected_bills = Vec::new();
        
        // Group transactions by merchant and analyze patterns
        let mut merchant_transactions: HashMap<String, Vec<Transaction>> = HashMap::new();
        
        for transaction in transactions {
            if transaction.amount > Decimal::ZERO {
                continue; // Skip income
            }
            
            let merchant = transaction.merchant_name
                .clone()
                .unwrap_or_else(|| transaction.name.clone());
            
            merchant_transactions.entry(merchant).or_default().push(transaction);
        }
        
        // Analyze each merchant for recurring patterns
        for (merchant, transactions) in merchant_transactions {
            if transactions.len() < 2 {
                continue; // Need at least 2 transactions to detect pattern
            }
            
            let pattern_analysis = self.ai_service
                .analyze_recurring_pattern(&transactions)
                .await?;
            
            if pattern_analysis.is_recurring {
                let bill = Bill {
                    id: Uuid::new_v4(),
                    name: self.generate_bill_name(&merchant, &pattern_analysis),
                    vendor: merchant,
                    amount: pattern_analysis.average_amount,
                    due_date: pattern_analysis.predicted_next_date,
                    frequency: pattern_analysis.frequency,
                    category: pattern_analysis.category,
                    account_id: transactions.first().map(|t| t.account_id),
                    autopay_enabled: false,
                    last_paid_date: transactions.iter()
                        .max_by_key(|t| t.date)
                        .map(|t| t.date),
                    last_paid_amount: transactions.last().map(|t| t.amount.abs()),
                    notes: None,
                    is_active: true,
                    created_at: Utc::now(),
                };
                
                self.storage.store_bill(&bill).await?;
                detected_bills.push(bill);
            }
        }
        
        Ok(detected_bills)
    }
    
    pub async fn schedule_bill_reminders(&self, user_id: Uuid) -> Result<()> {
        let bills = self.storage.get_active_bills(user_id).await?;
        let today = Utc::now().date_naive();
        
        for bill in bills {
            let days_until_due = (bill.due_date - today).num_days();
            
            // Schedule reminders based on bill amount and user preferences
            let reminder_days = if bill.amount > Decimal::from(1000) {
                vec![7, 3, 1] // Major bills: 1 week, 3 days, 1 day
            } else {
                vec![3, 1] // Regular bills: 3 days, 1 day
            };
            
            for reminder_day in reminder_days {
                if days_until_due == reminder_day {
                    self.notification_service.schedule_bill_reminder(
                        user_id,
                        &bill,
                        reminder_day,
                    ).await?;
                }
            }
            
            // Check for overdue bills
            if days_until_due < 0 {
                self.notification_service.send_overdue_bill_alert(
                    user_id,
                    &bill,
                    days_until_due.abs(),
                ).await?;
            }
        }
        
        Ok(())
    }
    
    pub async fn suggest_payment_optimizations(&self, user_id: Uuid) -> Result<Vec<PaymentOptimization>> {
        let bills = self.storage.get_active_bills(user_id).await?;
        let accounts = self.storage.get_user_accounts(user_id).await?;
        let mut optimizations = Vec::new();
        
        // Analyze payment timing
        for bill in &bills {
            // Suggest autopay for regular bills
            if !bill.autopay_enabled && matches!(bill.frequency, BillFrequency::Monthly) {
                optimizations.push(PaymentOptimization {
                    bill_id: bill.id,
                    optimization_type: PaymentOptimizationType::EnableAutopay,
                    description: format!(
                        "Enable autopay for {} to avoid late fees and save time",
                        bill.name
                    ),
                    potential_savings: Decimal::from(25), // Estimated late fee avoidance
                    priority: OptimizationPriority::Medium,
                });
            }
            
            // Suggest payment account optimization
            if let Some(account_id) = bill.account_id {
                if let Some(current_account) = accounts.iter().find(|a| a.id == account_id) {
                    // Suggest using high-yield account or credit card with rewards
                    if let Some(better_account) = self.find_better_payment_account(&accounts, &bill) {
                        optimizations.push(PaymentOptimization {
                            bill_id: bill.id,
                            optimization_type: PaymentOptimizationType::ChangePaymentAccount,
                            description: format!(
                                "Pay {} from {} instead of {} for better rewards/rates",
                                bill.name,
                                better_account.name,
                                current_account.name
                            ),
                            potential_savings: self.calculate_account_savings(&bill, &better_account, &current_account),
                            priority: OptimizationPriority::Low,
                        });
                    }
                }
            }
        }
        
        // Suggest bill consolidation opportunities
        let consolidation_opportunities = self.find_consolidation_opportunities(&bills).await?;
        optimizations.extend(consolidation_opportunities);
        
        Ok(optimizations)
    }
    
    fn generate_bill_name(&self, merchant: &str, analysis: &RecurringPatternAnalysis) -> String {
        match analysis.category.as_str() {
            "utilities" => format!("{} Utility Bill", merchant),
            "insurance" => format!("{} Insurance Premium", merchant),
            "subscription" => format!("{} Subscription", merchant),
            "loan" => format!("{} Loan Payment", merchant),
            _ => format!("{} Bill", merchant),
        }
    }
    
    fn find_better_payment_account(&self, accounts: &[Account], bill: &Bill) -> Option<&Account> {
        // Logic to find account with better rewards, lower fees, etc.
        accounts.iter()
            .filter(|a| a.account_type == AccountType::CreditCard)
            .max_by_key(|a| {
                // This would calculate reward points/cashback for the bill category
                0 // Placeholder
            })
    }
    
    fn calculate_account_savings(&self, bill: &Bill, better_account: &Account, current_account: &Account) -> Decimal {
        // Calculate potential savings from switching payment accounts
        // This would consider rewards, fees, interest rates, etc.
        bill.amount * Decimal::from_f32(0.02).unwrap() // 2% rewards example
    }
}

#[derive(Debug, Clone)]
pub struct RecurringPatternAnalysis {
    pub is_recurring: bool,
    pub frequency: BillFrequency,
    pub average_amount: Decimal,
    pub predicted_next_date: NaiveDate,
    pub category: String,
    pub confidence_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentOptimization {
    pub bill_id: Uuid,
    pub optimization_type: PaymentOptimizationType,
    pub description: String,
    pub potential_savings: Decimal,
    pub priority: OptimizationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentOptimizationType {
    EnableAutopay,
    ChangePaymentAccount,
    ConsolidateBills,
    NegotiateBetterRate,
    SwitchToAnnualPayment,
}
```

## API Endpoints

### Financial Management Endpoints

```rust
pub fn create_financial_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/accounts/link", post(link_bank_account))
        .route("/api/v1/accounts", get(get_accounts))
        .route("/api/v1/accounts/:id/balance", get(get_account_balance))
        .route("/api/v1/transactions/sync", post(sync_transactions))
        .route("/api/v1/transactions", get(get_transactions))
        .route("/api/v1/budgets", post(create_budget).get(get_budgets))
        .route("/api/v1/budgets/:id/report", get(get_budget_report))
        .route("/api/v1/budgets/:id/optimize", get(suggest_budget_optimizations))
        .route("/api/v1/investments/sync", post(sync_investments))
        .route("/api/v1/investments/analysis", get(get_portfolio_analysis))
        .route("/api/v1/bills/detect", post(detect_bills))
        .route("/api/v1/bills", get(get_bills))
        .route("/api/v1/bills/:id/optimize", get(get_payment_optimizations))
        .route("/api/v1/financial-summary", get(get_financial_summary))
}

#[derive(Deserialize)]
pub struct LinkAccountRequest {
    pub public_token: String,
}

pub async fn link_bank_account(
    State(state): State<AppState>,
    Json(request): Json<LinkAccountRequest>,
) -> Result<Json<Vec<Account>>, StatusCode> {
    let user_id = get_authenticated_user_id()?; // Extract from JWT
    
    let accounts = state.financial.banking_service
        .link_account(&request.public_token, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(accounts))
}

pub async fn get_financial_summary(
    State(state): State<AppState>,
) -> Result<Json<FinancialSummary>, StatusCode> {
    let user_id = get_authenticated_user_id()?;
    
    // Gather data from all financial services
    let accounts = state.financial.storage.get_user_accounts(user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let recent_transactions = state.financial.storage
        .get_recent_transactions(user_id, 30)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let active_budgets = state.financial.storage
        .get_active_budgets(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Calculate summary metrics
    let total_assets = accounts.iter()
        .filter(|a| matches!(a.account_type, AccountType::Checking | AccountType::Savings | AccountType::Investment))
        .map(|a| a.balance)
        .sum::<Decimal>();
    
    let total_debt = accounts.iter()
        .filter(|a| matches!(a.account_type, AccountType::CreditCard | AccountType::Loan | AccountType::Mortgage))
        .map(|a| a.balance)
        .sum::<Decimal>();
    
    let monthly_spending = recent_transactions.iter()
        .filter(|t| t.amount < Decimal::ZERO)
        .map(|t| t.amount.abs())
        .sum::<Decimal>();
    
    let summary = FinancialSummary {
        total_assets,
        total_debt,
        net_worth: total_assets - total_debt,
        monthly_spending,
        active_budgets: active_budgets.len(),
        accounts_count: accounts.len(),
        last_updated: Utc::now(),
    };
    
    Ok(Json(summary))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialSummary {
    pub total_assets: Decimal,
    pub total_debt: Decimal,
    pub net_worth: Decimal,
    pub monthly_spending: Decimal,
    pub active_budgets: usize,
    pub accounts_count: usize,
    pub last_updated: DateTime<Utc>,
}
```

## Security and Compliance

### 1. PCI DSS Compliance

```rust
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

pub struct FinancialSecurityService {
    encryption_key: LessSafeKey,
    audit_logger: AuditLogger,
}

impl FinancialSecurityService {
    pub fn encrypt_sensitive_data(&self, data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        let mut encrypted_data = data.to_vec();
        let nonce = self.generate_secure_nonce()?;
        
        self.encryption_key.seal_in_place_append_tag(
            nonce,
            Aad::empty(),
            &mut encrypted_data,
        )?;
        
        Ok(encrypted_data)
    }
    
    pub async fn log_financial_access(&self, user_id: Uuid, action: &str, resource: &str) -> Result<()> {
        self.audit_logger.log(AuditEvent {
            user_id,
            action: action.to_string(),
            resource: resource.to_string(),
            timestamp: Utc::now(),
            ip_address: None, // Would be captured from request context
            success: true,
        }).await
    }
}
```

### 2. Data Retention Policies

```rust
pub struct DataRetentionService {
    storage: Arc<FinancialStorage>,
    retention_config: RetentionConfig,
}

impl DataRetentionService {
    pub async fn cleanup_expired_data(&self) -> Result<()> {
        let cutoff_date = Utc::now() - Duration::days(self.retention_config.transaction_retention_days);
        
        // Archive old transactions
        let old_transactions = self.storage
            .get_transactions_before_date(cutoff_date.date_naive())
            .await?;
        
        for transaction in old_transactions {
            self.storage.archive_transaction(&transaction).await?;
            self.storage.delete_transaction(transaction.id).await?;
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RetentionConfig {
    pub transaction_retention_days: i64,
    pub budget_retention_days: i64,
    pub investment_retention_days: i64,
    pub audit_log_retention_days: i64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            transaction_retention_days: 2555, // 7 years for tax purposes
            budget_retention_days: 1095,      // 3 years
            investment_retention_days: 3650,  // 10 years
            audit_log_retention_days: 2555,   // 7 years
        }
    }
}
```

## Performance Optimizations

### 1. Background Data Synchronization

```rust
pub struct FinancialSyncScheduler {
    banking_service: Arc<BankingService>,
    investment_service: Arc<InvestmentService>,
    bill_service: Arc<BillService>,
}

impl FinancialSyncScheduler {
    pub async fn start_sync_tasks(&self) {
        // Sync transactions every 4 hours
        let banking_service = self.banking_service.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(14400));
            loop {
                interval.tick().await;
                if let Err(e) = banking_service.sync_all_user_transactions().await {
                    tracing::error!("Failed to sync transactions: {}", e);
                }
            }
        });
        
        // Update investment prices every hour during market hours
        let investment_service = self.investment_service.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                if Self::is_market_hours() {
                    if let Err(e) = investment_service.update_all_prices().await {
                        tracing::error!("Failed to update investment prices: {}", e);
                    }
                }
            }
        });
    }
    
    fn is_market_hours() -> bool {
        // Check if current time is during market hours (9:30 AM - 4:00 PM ET)
        let now = Utc::now().with_timezone(&chrono_tz::US::Eastern);
        let hour = now.hour();
        let minute = now.minute();
        
        match now.weekday() {
            chrono::Weekday::Sat | chrono::Weekday::Sun => false,
            _ => {
                let current_minutes = hour * 60 + minute;
                let market_open = 9 * 60 + 30; // 9:30 AM
                let market_close = 16 * 60;    // 4:00 PM
                current_minutes >= market_open && current_minutes <= market_close
            }
        }
    }
}
```

## Testing Strategy

### 1. Financial Integration Tests

```rust
#[cfg(test)]
mod financial_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_budget_creation_and_tracking() {
        let test_env = setup_financial_test_env().await;
        
        // Create budget from historical data
        let budget = test_env.budget_service
            .create_budget_from_spending_history(
                test_user_id(),
                BudgetPeriod::Monthly,
                3, // 3 months of history
            )
            .await
            .unwrap();
        
        assert!(!budget.categories.is_empty());
        assert!(budget.total_budget > Decimal::ZERO);
        
        // Track spending
        let report = test_env.budget_service
            .track_spending_against_budget(test_user_id(), budget.id)
            .await
            .unwrap();
        
        assert_eq!(report.budget_id, budget.id);
    }
    
    #[tokio::test]
    async fn test_transaction_categorization() {
        let test_env = setup_financial_test_env().await;
        
        let transaction = create_test_transaction("AMAZON.COM", Decimal::from(-50));
        let categorized = test_env.banking_service
            .enhance_transaction_with_ai(transaction)
            .await
            .unwrap();
        
        assert!(categorized.custom_category.is_some());
        assert!(!categorized.tags.is_empty());
    }
    
    #[tokio::test]
    async fn test_bill_detection() {
        let test_env = setup_financial_test_env().await;
        
        // Create recurring transactions
        create_recurring_test_transactions(&test_env, "Netflix", 3).await;
        
        let detected_bills = test_env.bill_service
            .detect_bills_from_transactions(test_user_id())
            .await
            .unwrap();
        
        assert!(!detected_bills.is_empty());
        assert!(detected_bills.iter().any(|b| b.vendor.contains("Netflix")));
    }
}
```

Phase 3 implementation provides comprehensive financial management capabilities while maintaining strong security and privacy standards. The modular design allows for gradual rollout and easy integration with various financial institutions and services. The AI-powered insights help users make better financial decisions and optimize their spending patterns.