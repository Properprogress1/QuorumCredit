#[cfg(test)]
mod syndication_tests {
    use crate::{
        LoanSyndication, QuorumCreditContract, QuorumCreditContractClient, SyndicationConfig,
        SyndicationMember, SyndicationRole, SyndicationStatus,
    };
    use soroban_sdk::{testutils::Address as _, Address, Vec};

    fn setup_syndication() -> (
        Env,
        QuorumCreditContractClient<'static>,
        Address,
        Address,
        Address,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let deployer = Address::generate(&env);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        let admins = Vec::from_array(&env, [admin1.clone(), admin2.clone()]);
        let token = env
            .register_stellar_asset_contract_v2(admin1.clone())
            .address();

        let contract_id = env.register_contract(None, QuorumCreditContract);
        let client = QuorumCreditContractClient::new(&env, &contract_id);
        client.initialize(&deployer, &admins, &2, &token);

        (env, client, admin1, admin2, deployer)
    }

    #[test]
    fn test_create_syndication() {
        let (env, client, _admin1, _admin2, _deployer) = setup_syndication();
        let creator = Address::generate(&env);
        let loan_purpose = soroban_sdk::String::from_str(&env, "Business expansion");
        let token = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let total_amount = 100_000_000; // 10 XLM

        let syndication_id = client
            .create_syndication(&creator, &loan_purpose, &token, &total_amount)
            .unwrap();

        assert_eq!(syndication_id, 1);

        let syndication: LoanSyndication = client
            .get_syndication(syndication_id)
            .unwrap()
            .try_into()
            .unwrap();

        assert_eq!(syndication.syndication_id, syndication_id);
        assert_eq!(syndication.status, SyndicationStatus::Forming);
        assert_eq!(syndication.total_amount, total_amount);
        assert_eq!(syndication.members.len(), 0);
    }

    #[test]
    fn test_join_syndication() {
        let (env, client, _admin1, _admin2, _deployer) = setup_syndication();
        let creator = Address::generate(&env);
        let loan_purpose = soroban_sdk::String::from_str(&env, "Business expansion");
        let token = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let total_amount = 100_000_000;

        let syndication_id = client
            .create_syndication(&creator, &loan_purpose, &token, &total_amount)
            .unwrap();

        let member1 = Address::generate(&env);
        client
            .join_syndication(
                &syndication_id,
                &member1,
                SyndicationRole::LeadBorrower,
                &5000,
                &10_000_000,
                &5_000_000,
            )
            .unwrap();

        let syndication: LoanSyndication = client
            .get_syndication(syndication_id)
            .unwrap()
            .try_into()
            .unwrap();

        assert_eq!(syndication.members.len(), 1);
        assert_eq!(syndication.total_collateral, 10_000_000);
    }

    #[test]
    fn test_join_syndication_invalid_share() {
        let (env, client, _admin1, _admin2, _deployer) = setup_syndication();
        let creator = Address::generate(&env);
        let loan_purpose = soroban_sdk::String::from_str(&env, "Business expansion");
        let token = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let total_amount = 100_000_000;

        let syndication_id = client
            .create_syndication(&creator, &loan_purpose, &token, &total_amount)
            .unwrap();

        let member1 = Address::generate(&env);
        let result = client.try_join_syndication(
            &syndication_id,
            &member1,
            SyndicationRole::LeadBorrower,
            &15000, // Invalid: > 10000
            &10_000_000,
            &5_000_000,
        );

        assert_eq!(result, Err(Ok(crate::ContractError::InvalidSyndicationShare)));
    }

    #[test]
    fn test_approve_syndication() {
        let (env, client, _admin1, _admin2, _deployer) = setup_syndication();
        let creator = Address::generate(&env);
        let loan_purpose = soroban_sdk::String::from_str(&env, "Business expansion");
        let token = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let total_amount = 100_000_000;

        let syndication_id = client
            .create_syndication(&creator, &loan_purpose, &token, &total_amount)
            .unwrap();

        let member1 = Address::generate(&env);
        client
            .join_syndication(
                &syndication_id,
                &member1,
                SyndicationRole::LeadBorrower,
                &5000,
                &10_000_000,
                &5_000_000,
            )
            .unwrap();

        client
            .approve_syndication(&syndication_id, &member1)
            .unwrap();

        let syndication: LoanSyndication = client
            .get_syndication(syndication_id)
            .unwrap()
            .try_into()
            .unwrap();

        assert_eq!(syndication.approval_count, 1);

        let member: SyndicationMember = client
            .get_syndication_member(syndication_id, member1)
            .unwrap()
            .try_into()
            .unwrap();

        assert!(member.approved);
    }

    #[test]
    fn test_leave_syndication() {
        let (env, client, _admin1, _admin2, _deployer) = setup_syndication();
        let creator = Address::generate(&env);
        let loan_purpose = soroban_sdk::String::from_str(&env, "Business expansion");
        let token = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let total_amount = 100_000_000;

        let syndication_id = client
            .create_syndication(&creator, &loan_purpose, &token, &total_amount)
            .unwrap();

        let member1 = Address::generate(&env);
        client
            .join_syndication(
                &syndication_id,
                &member1,
                SyndicationRole::LeadBorrower,
                &5000,
                &10_000_000,
                &5_000_000,
            )
            .unwrap();

        client.leave_syndication(&syndication_id, &member1).unwrap();

        let syndication: LoanSyndication = client
            .get_syndication(syndication_id)
            .unwrap()
            .try_into()
            .unwrap();

        assert_eq!(syndication.members.len(), 0);
        assert_eq!(syndication.status, SyndicationStatus::Cancelled);
    }

    #[test]
    fn test_cancel_syndication() {
        let (env, client, _admin1, _admin2, _deployer) = setup_syndication();
        let creator = Address::generate(&env);
        let loan_purpose = soroban_sdk::String::from_str(&env, "Business expansion");
        let token = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let total_amount = 100_000_000;

        let syndication_id = client
            .create_syndication(&creator, &loan_purpose, &token, &total_amount)
            .unwrap();

        let lead_borrower = Address::generate(&env);
        client
            .join_syndication(
                &syndication_id,
                &lead_borrower,
                SyndicationRole::LeadBorrower,
                &5000,
                &10_000_000,
                &5_000_000,
            )
            .unwrap();

        client
            .cancel_syndication(&syndication_id, &lead_borrower)
            .unwrap();

        let syndication: LoanSyndication = client
            .get_syndication(syndication_id)
            .unwrap()
            .try_into()
            .unwrap();

        assert_eq!(syndication.status, SyndicationStatus::Cancelled);
    }

    #[test]
    fn test_cancel_syndication_unauthorized() {
        let (env, client, _admin1, _admin2, _deployer) = setup_syndication();
        let creator = Address::generate(&env);
        let loan_purpose = soroban_sdk::String::from_str(&env, "Business expansion");
        let token = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let total_amount = 100_000_000;

        let syndication_id = client
            .create_syndication(&creator, &loan_purpose, &token, &total_amount)
            .unwrap();

        let lead_borrower = Address::generate(&env);
        client
            .join_syndication(
                &syndication_id,
                &lead_borrower,
                SyndicationRole::LeadBorrower,
                &5000,
                &10_000_000,
                &5_000_000,
            )
            .unwrap();

        let unauthorized = Address::generate(&env);
        let result = client.try_cancel_syndication(&syndication_id, &unauthorized);

        assert_eq!(result, Err(Ok(crate::ContractError::Unauthorized)));
    }

    #[test]
    fn test_set_syndication_config() {
        let (env, client, admin1, admin2, _deployer) = setup_syndication();

        let config = SyndicationConfig {
            max_members: 15,
            min_members: 3,
            min_approval_percentage: 8000,
            max_loan_amount: 2_000_000_000_000,
            syndication_fee_bps: 150,
        };

        client
            .set_syndication_config(
                &Vec::from_array(&env, [admin1.clone(), admin2.clone()]),
                &config,
            )
            .unwrap();

        let retrieved_config = client.get_syndication_config_view();
        assert_eq!(retrieved_config.max_members, 15);
        assert_eq!(retrieved_config.min_members, 3);
    }

    #[test]
    fn test_set_syndication_config_invalid() {
        let (env, client, admin1, admin2, _deployer) = setup_syndication();

        let config = SyndicationConfig {
            max_members: 5,
            min_members: 10, // Invalid: min > max
            min_approval_percentage: 8000,
            max_loan_amount: 2_000_000_000_000,
            syndication_fee_bps: 150,
        };

        let result = client.try_set_syndication_config(
            &Vec::from_array(&env, [admin1.clone(), admin2.clone()]),
            &config,
        );

        assert_eq!(result, Err(Ok(crate::ContractError::InvalidSyndicationConfig)));
    }

    #[test]
    fn test_get_syndication_count() {
        let (env, client, _admin1, _admin2, _deployer) = setup_syndication();
        let creator = Address::generate(&env);
        let loan_purpose = soroban_sdk::String::from_str(&env, "Business expansion");
        let token = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let total_amount = 100_000_000;

        assert_eq!(client.get_syndication_count(), 0);

        client
            .create_syndication(&creator, &loan_purpose, &token, &total_amount)
            .unwrap();

        assert_eq!(client.get_syndication_count(), 1);
    }

    #[test]
    fn test_syndication_status_transitions() {
        let (env, client, _admin1, _admin2, _deployer) = setup_syndication();
        let creator = Address::generate(&env);
        let loan_purpose = soroban_sdk::String::from_str(&env, "Business expansion");
        let token = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let total_amount = 100_000_000;

        let syndication_id = client
            .create_syndication(&creator, &loan_purpose, &token, &total_amount)
            .unwrap();

        let syndication: LoanSyndication = client
            .get_syndication(syndication_id)
            .unwrap()
            .try_into()
            .unwrap();

        assert_eq!(syndication.status, SyndicationStatus::Forming);
    }

    #[test]
    fn test_default_syndication_config() {
        let config = crate::types::DEFAULT_SYNDICATION_CONFIG;
        assert_eq!(config.max_members, 10);
        assert_eq!(config.min_members, 2);
        assert_eq!(config.min_approval_percentage, 7500);
    }
}
