ALTER TABLE developer_workspaces
    ADD COLUMN logo_jpeg BYTEA,
    ADD COLUMN logo_version UUID,
    ADD CONSTRAINT workspace_logo_payload CHECK (
        (logo_jpeg IS NULL AND logo_version IS NULL)
        OR (logo_jpeg IS NOT NULL AND logo_version IS NOT NULL AND octet_length(logo_jpeg) BETWEEN 1 AND 65536)
    );
