import { describe, it, expect } from 'vitest'

describe('Financial Calculations', () => {
  describe('Debt Payoff Calculation', () => {
    it('should calculate payoff months for simple debt', () => {
      // Formula: n = -ln(1 - r*B/M) / ln(1+r)
      // For $5000 balance, 22% APR, $150 payment: ~51.99 months
      const balance = 5000
      const apr = 0.22
      const monthlyPayment = 150
      const monthlyRate = apr / 12

      const months = -Math.log(1 - (monthlyRate * balance) / monthlyPayment) / Math.log(1 + monthlyRate)

      expect(months).toBeGreaterThan(50)
      expect(months).toBeLessThan(55)
      expect(months).toBeCloseTo(51.99, 1)
    })

    it('should calculate monthly interest correctly', () => {
      const balance = 5000
      const apr = 0.22
      const monthlyRate = apr / 12

      const monthlyInterest = balance * monthlyRate

      expect(monthlyInterest).toBeCloseTo(91.67, 1)
    })

    it('should handle zero interest rate', () => {
      const balance = 1000
      const monthlyPayment = 100

      // With zero interest, it's just balance / payment
      const months = balance / monthlyPayment

      expect(months).toBe(10)
    })

    it('should calculate interest saved from accelerated payoff', () => {
      const balance = 5000
      const apr = 0.22
      const baselinePayment = 100
      const acceleratedPayment = 150

      const monthlyRate = apr / 12

      const baselineMonths = -Math.log(1 - (monthlyRate * balance) / baselinePayment) / Math.log(1 + monthlyRate)
      const acceleratedMonths = -Math.log(1 - (monthlyRate * balance) / acceleratedPayment) / Math.log(1 + monthlyRate)

      // Total paid = sum of all payments
      const baselineInterest = baselineMonths * baselinePayment - balance
      const acceleratedInterest = acceleratedMonths * acceleratedPayment - balance
      const interestSaved = baselineInterest - acceleratedInterest

      // Higher payment reduces payoff time significantly, saving substantial interest
      expect(interestSaved).toBeGreaterThan(0)
      expect(interestSaved).toBeGreaterThan(5000) // Substantial savings
    })
  })

  describe('Debt Ratio Calculation', () => {
    it('should calculate debt to assets ratio', () => {
      const totalDebt = 10000
      const totalAssets = 50000

      const debtRatio = totalDebt / totalAssets

      expect(debtRatio).toBe(0.2)
    })

    it('should handle zero debt', () => {
      const totalDebt = 0
      const totalAssets = 50000

      const debtRatio = totalDebt / totalAssets

      expect(debtRatio).toBe(0)
    })

    it('should handle equal debt and assets', () => {
      const totalDebt = 50000
      const totalAssets = 50000

      const debtRatio = totalDebt / totalAssets

      expect(debtRatio).toBe(1)
    })
  })

  describe('Interest as Percentage of Income', () => {
    it('should calculate interest as percentage of income', () => {
      const interestPaid = 500
      const income = 5000

      const percentage = (interestPaid / income) * 100

      expect(percentage).toBe(10)
    })

    it('should handle zero income', () => {
      const interestPaid = 500
      const income = 0

      const percentage = income > 0 ? (interestPaid / income) * 100 : 0

      expect(percentage).toBe(0)
    })
  })
})
