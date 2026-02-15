-- Add migration script here

ALTER TABLE portfolio_variants
  ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'portfolio_variants'
      AND constraint_type = 'UNIQUE'
      AND constraint_name = 'uq_portfolio_variants_name'
  ) THEN
    ALTER TABLE portfolio_variants
      ADD CONSTRAINT uq_portfolio_variants_name UNIQUE (name);
  END IF;
END$$;

DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'portfolio_variants'
      AND column_name = 'strategy_id'
      AND is_nullable = 'NO'
  ) THEN
    ALTER TABLE portfolio_variants ALTER COLUMN strategy_id DROP NOT NULL;
  END IF;
END$$;


DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name='variant_constraints' AND column_name='max_gross_exposure'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name='variant_constraints' AND column_name='gross_cap'
  ) THEN
    ALTER TABLE variant_constraints RENAME COLUMN max_gross_exposure TO gross_cap;
  END IF;
END$$;

DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name='variant_constraints' AND column_name='max_single_name_abs'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name='variant_constraints' AND column_name='max_pos'
  ) THEN
    ALTER TABLE variant_constraints RENAME COLUMN max_single_name_abs TO max_pos;
  END IF;
END$$;

DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name='variant_constraints' AND column_name='max_turnover_daily'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name='variant_constraints' AND column_name='turnover_cap'
  ) THEN
    ALTER TABLE variant_constraints RENAME COLUMN max_turnover_daily TO turnover_cap;
  END IF;
END$$;

ALTER TABLE variant_constraints
  ADD COLUMN IF NOT EXISTS vol_target   DOUBLE PRECISION,
  ADD COLUMN IF NOT EXISTS lambda_risk  DOUBLE PRECISION,
  ADD COLUMN IF NOT EXISTS gamma_cost   DOUBLE PRECISION,
  ADD COLUMN IF NOT EXISTS ignore_open  BOOLEAN NOT NULL DEFAULT FALSE;

