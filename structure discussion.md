One further thing how SAP grou these doctypes, because in order to create Finance I need these Business Partner (Customer/ Suppliers), I thiking how my apps should be structured.

11:58 PM
Great question. This is about app and module boundaries — getting this right from day one determines how maintainable the system is as it grows.

How SAP Structures This
SAP organizes everything into Application Components which map roughly to what we would call apps or modules. The relevant ones for your context:

FI — Financial Accounting
    FI-GL    General Ledger
    FI-AR    Accounts Receivable
    FI-AP    Accounts Payable
    FI-AA    Asset Accounting
    FI-TV    Travel Management

CO — Controlling
    CO-CCA   Cost Center Accounting
    CO-PC    Product Costing
    CO-PA    Profitability Analysis

MM — Materials Management
    MM-PUR   Purchasing
    MM-IM    Inventory Management
    MM-IV    Invoice Verification

SD — Sales and Distribution
    SD-BF    Basic Functions
    SD-BIL   Billing

BP — Business Partner (cross-cutting)
    Shared across FI-AR, FI-AP, SD, MM
    Does not belong to any single module
The critical insight from SAP's structure is Business Partner is explicitly cross-cutting. It does not live inside FI. It does not live inside SD. It is a shared foundation that both consume. SAP learned this the hard way — in older versions Customer lived in SD and Vendor lived in MM/FI, causing exactly the duplication problem we discussed. S/4HANA moved BP out into its own layer.

How ERPNext Structures This
ERPNext has these apps and modules:

frappe (framework)
    └── erpnext (one giant app)
            Accounts
            Buying
            Selling
            Stock
            HR
            Manufacturing
            CRM
            Projects
The problem — Customer lives in Selling, Supplier lives in Buying, but both are referenced in Accounts. This creates circular dependencies and is precisely why ERPNext has the awkward party_type string reference instead of a clean foreign key. The architecture drove the bad data model.

What SpotLedger Should Do
You have a clean slate. Structure it like this:

spotledger (framework — your Frappe-in-Rust equivalent)
    │
    ├── core (app)
    │       Currency
    │       Company
    │       Fiscal Year
    │       Accounting Period
    │       Cost Center
    │       Settings
    │
    ├── party (app)               ← standalone, no dependencies on other apps
    │       Party
    │       Party Role Config
    │       Payment Terms
    │       Address
    │       Contact
    │
    ├── accounts (app)            ← depends on core + party
    │       Chart of Accounts
    │       Account Group
    │       Account
    │       Account Company
    │       Financial Statement Version
    │       FSV Node
    │       FSV Account Mapping
    │       GL Entry
    │       Journal Entry
    │
    ├── inventory (app)           ← depends on core + party + accounts
    │       Item
    │       Warehouse
    │       Stock Entry
    │       Stock Ledger Entry
    │
    ├── transactions (app)        ← depends on core + party + accounts
    │       Invoice
    │       Payment
    │       Payment Reconciliation
    │       Tax Rule
    │
    └── reports (app)             ← depends on accounts + transactions
            Trial Balance
            Balance Sheet
            Profit and Loss
            AR Aging
            AP Aging
            Cash Flow
The Dependency Rule
The direction of dependency must always flow downward and never upward:

framework
    ↓
core
    ↓
party
    ↓
accounts
    ↓
inventory / transactions
    ↓
reports
accounts can reference party. party can never reference accounts. This is the rule that prevents circular dependencies and is the reason SAP eventually extracted BP as its own layer.

Why Party Must Be Its Own App
This is the key structural decision. Party must sit below accounts in the dependency chain because:

accounts needs to reference Party on GL Entry
inventory needs Party for supplier on purchase receipts
transactions needs Party for customer on invoices
Future hr app will need Party for employees
Future crm app will need Party for prospects
If Party lived inside accounts, then inventory and transactions would depend on accounts just to get a Party reference — even if they did not need any accounting functionality. Clean separation avoids this entirely.

Practical Answer for Where to Start
When you sit down to start coding, create in this order:

1. spotledger framework (DocType engine, routing, auth)
2. core app (Currency, Company, Fiscal Year, Period, Cost Center)
3. party app (Party, Party Role Config, Payment Terms)
4. accounts app (CoA, Account, GL Entry, Journal Entry, FSV)
5. transactions app (Invoice, Payment)
6. reports app
Each app is a folder with its own DocTypes, controllers, and SurrealQL functions. No app reaches up the dependency chain. This is the structure that scales cleanly from one-company SME to multi-entity group without refactoring.

You said party app, What SAP calls it, Isnt this Business Partner App

12:02 AM
Yes, exactly. SAP calls it Business Partner and it has its own transaction code: BP.

In SAP S/4HANA it is formally:

SAP Business Partner
Transaction Code: BP
Application Component: CA-BP
CA = Cross Application
The CA prefix is the telling detail. Cross Application — SAP explicitly placed it outside FI, outside SD, outside MM. It sits at the cross-application foundation layer specifically because every module needs it.

So in SpotLedger the correct naming is:

spotledger (framework)
    │
    ├── core
    ├── business_partner          ← not "party", this is the correct name
    │       Party (the DocType can still be called Party internally)
    │       Party Role Config
    │       Payment Terms
    │       Address
    │       Contact
    │
    ├── accounts
    ├── inventory
    ├── transactions
    └── reports
The app is called Business Partner. The DocType inside it can remain Party — because Party is the cleaner, less SAP-specific name for the actual record, and it is what your controllers and SurrealQL will reference everywhere. This mirrors exactly what SAP did — the application component is called Business Partner but the underlying object in S/4HANA tables is BUT000 which is still essentially a partner/party record.

So to be precise:

Level	Name	Reasoning
App / Module	Business Partner	SAP convention, cross-cutting, correct
DocType	Party	Clean, LLM-friendly, avoids SAP jargon in code
GL Entry field	party	Concise, unambiguous in query context
User-facing label	Business Partner	What users see in the UI
One name for the developer, one name for the user, same underlying record.