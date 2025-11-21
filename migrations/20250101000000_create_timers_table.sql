-- Create timers table
CREATE TABLE timers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    execute_at TIMESTAMPTZ NOT NULL,
    callback_url TEXT NOT NULL,
    callback_headers JSONB,
    callback_payload JSONB,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    last_error TEXT,
    executed_at TIMESTAMPTZ,
    metadata JSONB,

    CONSTRAINT valid_status CHECK (status IN ('pending', 'executing', 'completed', 'failed', 'canceled')),
    CONSTRAINT future_execute_at CHECK (execute_at > created_at)
);

-- Create indexes for performance
CREATE INDEX idx_timers_execute_at_status ON timers(execute_at, status)
    WHERE status = 'pending';

CREATE INDEX idx_timers_status ON timers(status);

CREATE INDEX idx_timers_created_at ON timers(created_at DESC);

-- Create updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_timers_updated_at BEFORE UPDATE ON timers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
