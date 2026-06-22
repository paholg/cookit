CREATE TABLE user_passkeys (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id text NOT NULL UNIQUE,
    passkey JSONB NOT NULL
);

SELECT
    diesel_manage_updated_at('user_passkeys');

CREATE TABLE user_passkey_registrations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    passkey_registration JSONB NOT NULL
);

SELECT
    diesel_manage_updated_at('user_passkey_registrations');

CREATE TABLE user_passkey_authentications (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    passkey_authentication JSONB NOT NULL
);

SELECT
    diesel_manage_updated_at('user_passkey_authentications');
