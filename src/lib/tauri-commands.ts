import { invoke } from '@tauri-apps/api/core'

// Error handling
export enum ErrorKind {
  Database = 'Database',
  SimpleFin = 'SimpleFin',
  Llm = 'Llm',
  Validation = 'Validation',
  Config = 'Config',
  Internal = 'Internal',
  Keychain = 'Keychain',
  NotFound = 'NotFound',
}

export interface AppError {
  error: ErrorKind
  details: string
}

// Account types
export enum AccountType {
  Checking = 'checking',
  Savings = 'savings',
  CreditCard = 'credit_card',
  Loan = 'loan',
}

// Models
export interface Account {
  id: string
  simplefin_account_id: string | null
  name: string
  account_type: AccountType
  organization: string | null
  balance: number
  last_updated: string // RFC3339
}

export interface RawTransaction {
  id: string
  account_id: string
  posted_date: string
  amount: number
  merchant: string | null
  description: string
  transaction_type: string
  imported_at: string
}

export interface CategorizedTransaction {
  id: string
  category: string
  secondary_category: string | null
  confidence: number
  note: string | null
  categorized_at: string
  is_manual: boolean
}

export interface DebtAccount {
  id: string
  simplefin_account_id: string | null
  name: string
  account_type: AccountType
  current_balance: number
  interest_rate: number
  minimum_payment: number | null
  last_updated: string
}

export type Period = 'week' | 'month'

export interface DailyMetrics {
  date: string  // YYYY-MM-DD
  income: number
  spending: number
  debt_paydown: number
  interest_paid: number
}

export interface DashboardMetrics {
  period: Period
  period_start: string  // ISO 8601 date (YYYY-MM-DD)
  period_end: string    // ISO 8601 date (YYYY-MM-DD)
  income: number
  spending: number
  debt_paydown: number
  interest_paid: number
  debt_ratio: number
  interest_as_pct_income: number  // percentage 0.0..100.0+
  sparkline_data: DailyMetrics[]
  last_sync: string | null  // RFC3339 timestamp
}

export interface SyncStatus {
  in_progress: boolean
  last_sync: string | null
  last_error: string | null
  transaction_count: number
}

export interface Scenario {
  monthly_cut: number
  months_saved: number
  interest_saved: number
  new_payoff_months: number
}

export interface GetOpportunityScenariosResponse {
  scenarios: Scenario[]
  total_debt: number
  weighted_apr: number
}

// Requests
export interface GetDashboardMetricsRequest {
  period: Period
}

export interface GetTransactionsRequest {
  account_id?: string
  category?: string
  start_date?: string
  end_date?: string
  limit?: number
  offset?: number
}

export interface SetDebtTermsRequest {
  account_id: string
  interest_rate: number // As decimal (0.2199 = 21.99%)
  minimum_payment?: number
}

export interface RecategorizeTransactionRequest {
  transaction_id: string
  category: string
  secondary_category?: string
  note?: string
}

export interface SyncSimplefinRequest {
  days_back?: number  // Default: 90 days
}

export interface ClaimSetupTokenRequest {
  setup_token: string
}

export interface ClaimSetupTokenResponse {
  success: boolean
  message: string
}

export interface SimpleFINStatusResponse {
  connected: boolean
  account_count?: number
}

export interface DisconnectSimpleFINResponse {
  success: boolean
  message: string
}

export interface LlmConfig {
  ollama_url: string
  llm_model: string
  use_local_first: boolean
}

export interface SyncSettings {
  sync_frequency: 'manual' | 'on-open' | '12h' | '24h'
  backfill_days: number
  enable_background_sync: boolean
}

export interface UiPreferences {
  theme: 'light' | 'dark' | 'auto'
  currency: string
}

export interface AllSettings {
  llm_config?: LlmConfig
  sync_settings?: SyncSettings
  ui_preferences?: UiPreferences
}

export interface SaveSettingsResponse {
  success: boolean
  message: string
}

export interface SaveLlmConfigRequest {
  ollama_url: string
  llm_model: string
  use_local_first: boolean
  api_key?: string
}

export interface SaveSyncSettingsRequest {
  sync_frequency: 'manual' | 'on-open' | '12h' | '24h'
  backfill_days: number
  enable_background_sync: boolean
}

export interface SaveUiPreferencesRequest {
  theme: 'light' | 'dark' | 'auto'
  currency: string
}

export interface TransactionMappingSuggestion {
  transaction_id: string
  current_account_id: string
  merchant: string | null
  description: string
  amount: number
  posted_date: string
}

export interface GetTransactionMappingSuggestionsResponse {
  unmapped_transactions: TransactionMappingSuggestion[]
  available_accounts: Account[]
  mapping_required: boolean
}

export interface SubmitTransactionMappingsRequest {
  mappings: [string, string][]
}

export interface SubmitTransactionMappingsResponse {
  success: boolean
  message: string
  updated_count: number
}

// Command functions
export async function getDashboardMetrics(period: Period): Promise<DashboardMetrics> {
  return invoke<DashboardMetrics>('get_dashboard_metrics', { period })
}

export async function getTransactions(req: GetTransactionsRequest): Promise<RawTransaction[]> {
  return invoke<RawTransaction[]>('get_transactions', req as Record<string, unknown>)
}

export async function getAccounts(): Promise<Account[]> {
  return invoke<Account[]>('get_accounts')
}

export async function setDebtTerms(req: SetDebtTermsRequest): Promise<void> {
  return invoke<void>('set_debt_terms', req as unknown as Record<string, unknown>)
}

export async function recategorizeTransaction(req: RecategorizeTransactionRequest): Promise<void> {
  return invoke<void>('recategorize_transaction', req as unknown as Record<string, unknown>)
}

export async function claimSetupToken(req: ClaimSetupTokenRequest): Promise<ClaimSetupTokenResponse> {
  return invoke<ClaimSetupTokenResponse>('claim_setup_token', req as unknown as Record<string, unknown>)
}

export async function syncSimpleFin(req: SyncSimplefinRequest): Promise<SyncStatus> {
  return invoke<SyncStatus>('sync_simplefin', req as unknown as Record<string, unknown>)
}

export async function getSyncStatus(): Promise<SyncStatus> {
  return invoke<SyncStatus>('get_sync_status')
}

export async function shouldSyncOnOpen(): Promise<boolean> {
  return invoke<boolean>('should_sync_on_open')
}

export async function getOpportunityScenarios(): Promise<GetOpportunityScenariosResponse> {
  return invoke<GetOpportunityScenariosResponse>('get_opportunity_scenarios')
}

export async function getSimpleFINStatus(): Promise<SimpleFINStatusResponse> {
  return invoke<SimpleFINStatusResponse>('get_simplefin_status')
}

export async function disconnectSimpleFIN(): Promise<DisconnectSimpleFINResponse> {
  return invoke<DisconnectSimpleFINResponse>('disconnect_simplefin')
}

export async function saveLlmConfig(req: SaveLlmConfigRequest): Promise<SaveSettingsResponse> {
  return invoke<SaveSettingsResponse>('save_llm_config', req as unknown as Record<string, unknown>)
}

export async function saveSyncSettings(req: SaveSyncSettingsRequest): Promise<SaveSettingsResponse> {
  return invoke<SaveSettingsResponse>('save_sync_settings', req as unknown as Record<string, unknown>)
}

export async function saveUiPreferences(req: SaveUiPreferencesRequest): Promise<SaveSettingsResponse> {
  return invoke<SaveSettingsResponse>('save_ui_preferences', req as unknown as Record<string, unknown>)
}

export async function getSettings(): Promise<AllSettings> {
  return invoke<AllSettings>('get_settings')
}

export async function getTransactionMappingSuggestions(): Promise<GetTransactionMappingSuggestionsResponse> {
  return invoke<GetTransactionMappingSuggestionsResponse>('get_transaction_mapping_suggestions')
}

export async function submitTransactionMappings(req: SubmitTransactionMappingsRequest): Promise<SubmitTransactionMappingsResponse> {
  return invoke<SubmitTransactionMappingsResponse>('submit_transaction_mappings', req as unknown as Record<string, unknown>)
}
