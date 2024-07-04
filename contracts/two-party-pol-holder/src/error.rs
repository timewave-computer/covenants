use cosmwasm_std::StdError;
use covenant_utils::op_mode::ContractOperationError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    ContractOperationError(#[from] ContractOperationError),

    #[error("party allocations must add up to 1.0")]
    AllocationValidationError {},

    #[error("Ragequit penalty must be in range of [0.0, 1.0)")]
    RagequitPenaltyRangeError {},

    #[error("Ragequit penalty exceeds party allocation")]
    RagequitPenaltyExceedsPartyAllocationError {},

    #[error("unauthorized")]
    Unauthorized {},

    #[error("contract needs to be in ragequit or expired state in order to claim")]
    ClaimError {},

    #[error("covenant is not in active state")]
    NotActive {},

    #[error("unexpected reply id")]
    UnexpectedReplyId {},

    #[error("covenant is active but expired; tick to proceed")]
    Expired {},

    #[error("both parties have not deposited")]
    InsufficientDeposits {},

    #[error("failed to multiply amount by share")]
    FractionMulError {},

    #[error("expiry block is already past")]
    InvalidExpiryBlockHeight {},

    #[error("lockup deadline must be after the deposit deadline")]
    LockupValidationError {},

    #[error("cannot validate deposit and lockup expirations")]
    ExpirationValidationError {},

    #[error("deposit deadline is already past")]
    DepositDeadlineValidationError {},

    #[error("shares of covenant parties must add up to 1.0")]
    InvolvedPartiesConfigError {},

    #[error("unknown party")]
    PartyNotFound {},

    #[error("ragequit is disabled")]
    RagequitDisabled {},

    #[error("only covenant parties can initiate ragequit")]
    RagequitUnauthorized {},

    #[error("ragequit attempt with lockup period passed")]
    RagequitWithLockupPassed {},

    #[error("ragequit already active")]
    RagequitInProgress {},

    #[error("unauthorized to distribute explicitly defined denom")]
    UnauthorizedDenomDistribution {},

    #[error("A withdraw process already started")]
    WithdrawAlreadyStarted {},

    #[error("A withdraw process wasn't started yet")]
    WithdrawStateNotStarted {},

    #[error("Claimer already claimed his share")]
    PartyAllocationIsZero {},

    #[error("Party contribution cannot be zero")]
    PartyContributionConfigError {},
}
