Here’s a practical roadmap that builds BizTrace in vertical, testable increments.

## Phase 0 — Foundations

Goal: turn the current RTL prototype into a maintainable application shell.

- Split `main.rs` into `state`, `message`, `screens`, `money`, and `locale`.
- Establish unidirectional Iced state/message/update/view flow.
- Create shared RTL layout and typography helpers.
- Add Persian font assets and verify shaping on target platforms.
- Define conventions for IDs, timestamps, errors, and module boundaries.
- Add formatting, linting, and basic CI:
  - `cargo fmt --check`
  - `cargo clippy`
  - `cargo test`

Deliverable: the existing smoke-test interface still works after restructuring.

## Phase 1 — Core domain types

Goal: make invalid money and inventory operations difficult to express.

- Implement `Rial(i64)` with:
  - checked addition, subtraction, and multiplication
  - thousands separators
  - Persian-digit rendering
  - Toman display conversion
  - no floating-point conversions
- Define domain models:
  - `User`
  - `Customer`
  - `Product`
  - `Sale`
  - `SaleLine`
- Introduce strongly typed IDs where practical.
- Define validation rules:
  - nonnegative price and stock
  - positive sale quantities
  - unique usernames and SKUs
  - valid customer phone numbers
- Add unit tests for monetary calculations and validation.

Deliverable: tested domain logic independent of the GUI and database.

Important detail: Persian digits are `۰۱۲۳۴۵۶۷۸۹`, while `٠١٢٣٤٥٦٧٨٩` are Arabic-Indic digits.

## Phase 2 — SQLite persistence

Goal: persist and retrieve the core records safely.

- Add SQLx with SQLite and migrations.
- Create `0001_init.sql` for:
  - users
  - customers
  - products
  - sales
  - sale lines
  - application settings
- Enable foreign keys and suitable indexes.
- Add database initialization and automatic migrations.
- Create repository functions instead of putting SQL in screens.
- Store:
  - money as SQLite `INTEGER`
  - timestamps in a single Gregorian/UTC representation
  - password hashes as encoded strings
- Add integration tests using temporary databases.

Deliverable: restart-safe CRUD operations with migration coverage.

## Phase 3 — Authentication and authorization

Goal: support secure local access with enforced roles.

- Hash passwords using Argon2id.
- Add login and logout flows.
- Create the initial administrator safely.
- Define permissions centrally, for example:
  - admin: full access and user management
  - manager: sales, inventory, customers, reports
  - staff: restricted sales and customer operations
- Enforce permissions in application commands, not only by hiding buttons.
- Add password-change and account-disable features.
- Avoid storing plaintext passwords or sensitive values in logs.

Deliverable: authenticated sessions with tested permission boundaries.

## Phase 4 — Application shell and navigation

Goal: establish the final Persian-first desktop experience.

- Build the main shell:
  - RTL navigation
  - current-user display
  - page title and contextual actions
  - loading and error states
- Implement reusable widgets:
  - RTL text fields
  - money input/display
  - confirmation dialog
  - searchable table
  - empty state
  - Persian date display
- Ensure keyboard navigation and focus order make sense in RTL.
- Keep screen modules focused on presentation; business operations remain in services/repositories.

Deliverable: functional navigation between placeholder screens.

## Phase 5 — Customer management

Goal: deliver the first complete business workflow.

- Customer list with search and pagination/filtering.
- Create and edit customer records.
- Validate and normalize phone numbers.
- Customer detail page with purchase history.
- Prevent accidental deletion when sales reference a customer.
- Prefer archive/deactivate behavior over destructive deletion.

Deliverable: production-ready customer CRUD from UI to database.

## Phase 6 — Inventory and products

Goal: maintain a trustworthy product catalog and stock count.

- Product and SKU management.
- Price entry in Rial with optional Toman-oriented UI assistance.
- Stock adjustments with explicit reasons.
- Low-stock and out-of-stock indicators.
- Search and filtering.
- Add an inventory-movement ledger rather than silently overwriting stock.
- Use database transactions for every stock-changing operation.

Deliverable: auditable product and inventory management.

## Phase 7 — Sales and invoices

Goal: implement the central transactional workflow.

- Create a sale and select an optional customer.
- Search and add products.
- Change quantities and remove lines.
- Calculate subtotal, discount, and final total using checked `Rial` operations.
- Validate stock immediately before committing.
- Commit the invoice, line items, and inventory movements in one transaction.
- Display and print/export a Persian invoice.
- Define cancellation/refund behavior without deleting financial history.

Deliverable: atomic sales processing with correct totals and stock updates.

## Phase 8 — Dashboard and reports

Goal: provide useful business insights without corrupting financial meaning.

- Daily, weekly, and monthly sales summaries.
- Low-stock overview.
- Best-selling products.
- Customer purchase summaries.
- Jalali date filters with Gregorian storage/query boundaries.
- Centralize Toman rendering in the presentation layer.
- Add CSV or spreadsheet-friendly export if required.
- Test totals against known datasets.

Deliverable: reliable operational dashboard and reports.

## Phase 9 — Administration and settings

Goal: make the application manageable without editing files or databases.

- User creation and role assignment.
- Disable/reactivate accounts.
- Business information for invoice headers.
- Currency display preference:
  - values remain stored in Rial
  - reports may display Rial or Toman
- Database backup and restore.
- Record application and schema versions.
- Add an audit log for sensitive operations.

Deliverable: admin-controlled configuration and recoverability.

## Phase 10 — Hardening and release

Goal: produce an installable, supportable desktop application.

- Add tests at three levels:
  - unit tests for money, dates, and validation
  - database integration tests
  - end-to-end tests for critical workflows
- Test Persian text input, cursor behavior, selection, and mixed Persian/Latin text.
- Test on every supported operating system.
- Handle database corruption, migration failure, and insufficient disk space.
- Add structured logging without secrets or password hashes.
- Package fonts, migrations, and other assets.
- Produce signed installers where applicable.
- Document backup, restore, upgrades, and first-admin setup.

Deliverable: release candidate suitable for real data.

## Suggested milestone order

1. Application foundation  
2. `Rial` and locale tests  
3. Database and migrations  
4. Authentication  
5. Customers  
6. Inventory  
7. Sales  
8. Reports  
9. Administration  
10. Packaging and release  

The first meaningful MVP should include authentication, customers, inventory, atomic sales, basic invoices, and backups. Dashboard analytics and advanced reporting can follow once the transactional core is stable.