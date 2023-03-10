ALTER TABLE outputs DROP CONSTRAINT outputs_input_id_fkey;
ALTER TABLE outputs ADD CONSTRAINT outputs_input_id_fkey FOREIGN KEY (input_id) REFERENCES inputs(id) ON DELETE CASCADE;

ALTER TABLE inputs DROP CONSTRAINT inputs_build_id_fkey;
ALTER TABLE inputs ADD CONSTRAINT inputs_build_id_fkey FOREIGN KEY (build_id) REFERENCES builds(id) ON DELETE CASCADE;
