# Token Flow Diagram

This diagram illustrates how tokens move through the Alignment Protocol system.

```mermaid
flowchart TD
    %% Entities
    Contributor([Data Contributor])
    Validator([Validator])
    Protocol([Protocol])
    
    %% Token Types
    TempAlign[Temporary ALIGN]
    TempRep[Temporary REP]
    PermanentAlign[Permanent ALIGN]
    PermanentRep[Permanent REP]
    
    %% Actions
    A1[Submit Data to Topic]
    A2[Stake Tokens]
    A3[Vote on Submission]
    A4[Finalize Submission]
    A5[Finalize Vote]
    
    %% Token States
    Burned((Burned))
    
    %% Flows
    Contributor -->|Creates Submission| A1
    A1 -->|Mints 100| TempAlign
    
    subgraph "Staking Phase"
        TempAlign -->|Staked| A2
        A2 -->|Burned| Burned
        A2 -->|Minted 1:1| TempRep
    end
    
    subgraph "Voting Phase"
        Validator -->|Has| TempRep
        TempRep -->|Used for| A3
        A3 -->|Quadratic Voting Power| Protocol
    end
    
    subgraph "Finalization Phase"
        A4 -->|If Accepted| ConvertToAlign
        A4 -->|If Rejected| BurnTemp
        A5 -->|If Vote Correct| ConvertToRep
        A5 -->|If Vote Incorrect| BurnTempRep
        
        ConvertToAlign[Convert] -->|1:1 Ratio| PermanentAlign
        BurnTemp[Burn] --> Burned
        ConvertToRep[Convert] -->|1:1 Ratio| PermanentRep
        BurnTempRep[Burn] --> Burned
    end
    
    %% Outcome
    PermanentAlign -->|Owned by| Contributor
    PermanentRep -->|Owned by| Validator
    
    %% Styling
    classDef tempTokens fill:#f9d5e5,stroke:#333,stroke-width:1px
    classDef permTokens fill:#ade8f4,stroke:#333,stroke-width:1px
    classDef actions fill:#e2e2e2,stroke:#333,stroke-width:1px
    classDef entities fill:#d8f3dc,stroke:#333,stroke-width:1px
    classDef burned fill:#ff9999,stroke:#333,stroke-width:1px
    
    class TempAlign,TempRep tempTokens
    class PermanentAlign,PermanentRep permTokens
    class A1,A2,A3,A4,A5,ConvertToAlign,ConvertToRep,BurnTemp,BurnTempRep actions
    class Contributor,Validator,Protocol entities
    class Burned burned
```

## Token Flow Explanation

1. **Initial Token Issuance**:
   - Contributors receive Temporary ALIGN tokens (tempAlign) upon submitting data
   - 100 tempAlign tokens are minted per submission

2. **Staking Mechanism**:
   - Validators stake tempAlign tokens to receive Temporary REP tokens (tempRep)
   - tempAlign is burned during staking in a 1:1 conversion ratio
   - tempRep represents voting power in the validation process

3. **Voting Process**:
   - Validators use tempRep tokens to vote on submissions
   - Voting power scales quadratically (sqrt of tokens) to balance influence
   - Votes go through commit and reveal phases for honest participation

4. **Finalization Outcomes**:
   - For Contributors:
     - If submission is accepted: tempAlign tokens convert to permanent ALIGN
     - If submission is rejected: tempAlign tokens are burned
   - For Validators:
     - If they voted with consensus: tempRep tokens convert to permanent REP
     - If they voted against consensus: tempRep tokens are burned

5. **Permanent Tokens**:
   - Permanent ALIGN (for contributors) and REP (for validators) represent successfully validated participation
   - These tokens can be used for governance and revenue sharing