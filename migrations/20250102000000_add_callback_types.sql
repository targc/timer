-- Migration: Add callback type system for HTTP and NATS callbacks
-- This migration transforms the HTTP-only callback schema into a flexible
-- callback_type + callback_config schema that supports both HTTP and NATS.

-- Step 1: Add new columns (without dropping old ones yet)
ALTER TABLE timers
ADD COLUMN callback_type VARCHAR(10),
ADD COLUMN callback_config JSONB;

-- Step 2: Migrate existing data from old schema to new schema
-- All existing timers are HTTP callbacks, transform them to the new format
UPDATE timers SET
  callback_type = 'http',
  callback_config = jsonb_build_object(
    'type', 'http',
    'url', callback_url,
    'headers', COALESCE(callback_headers, 'null'::jsonb),
    'payload', COALESCE(callback_payload, 'null'::jsonb)
  );

-- Step 3: Add NOT NULL constraints after data migration
ALTER TABLE timers
ALTER COLUMN callback_type SET DEFAULT 'http',
ALTER COLUMN callback_type SET NOT NULL,
ALTER COLUMN callback_config SET NOT NULL;

-- Step 4: Add CHECK constraint for valid callback types
ALTER TABLE timers
ADD CONSTRAINT valid_callback_type CHECK (callback_type IN ('http', 'nats'));

-- Step 5: Create index on callback_type for filtering
CREATE INDEX idx_timers_callback_type ON timers(callback_type);

-- Step 6: Drop old columns (after data is migrated)
ALTER TABLE timers
DROP COLUMN callback_url,
DROP COLUMN callback_headers,
DROP COLUMN callback_payload;

-- Migration complete: timers table now uses callback_type and callback_config
