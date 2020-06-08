bitfield! {
    pub struct TpmAccess(u8);
    impl Debug;
    pub tpm_reg_validsts, _: 7;
    pub active_locality, set_active_locality: 5;
    pub been_seized, set_been_seized: 4;
    pub seize, set_seize: 3;
    pub pending_request, _: 2;
    pub request_use, set_request_use: 1;
    pub tpm_establishment, _: 0;
}

bitfield! {
    pub struct TpmStatus(u8);
    impl Debug;
    u8;
    pub sts_valid, _: 7;
    pub command_ready, set_command_ready: 6;
    pub tpm_go, set_tpm_go: 5;
    pub data_avail, _: 4;
    pub expect, _: 3;
    pub selftest_done, _: 2;
    pub response_retry, set_response_retry: 1;
    rsvd, _: 0;
}
