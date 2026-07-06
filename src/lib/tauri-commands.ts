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

export interface DashboardMetrics {
  period: Period
  income: number
  spending: number
  debt_paydown: number
  interest_paid: number
  debt_ratio: number
  last_sync: string | null
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

export async function syncSimpleFin(): Promise<SyncStatus> {
  return invoke<SyncStatus>('sync_simplefin')
}

export async function getSyncStatus(): Promise<SyncStatus> {
  return invoke<SyncStatus>('get_sync_status')
}

export async function getOpportunityScenarios(): Promise<GetOpportunityScenariosResponse> {
  return invoke<GetOpportunityScenariosResponse>('get_opportunity_scenarios')
}
