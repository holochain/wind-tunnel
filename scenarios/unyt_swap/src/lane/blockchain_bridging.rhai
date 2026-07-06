// Lane unit the deposit credits; must equal WHOT_UNIT_INDEX in the scenario.
let unyt_index = "1";
let state = #{
    "0xe3e064e3c2eef66cb93da8d8114f5084e92f48d6": "0",
    "0xeth_mainnet": "0",
    "0xpolygon_mainnet": "0"
};
// Function definitions first
fn deposit_logic(consumed_inputs, state, unyt_index) {
    let unyt_allocation = [];
    // use the output of this agreement to update it based on the input
    for pod_link in consumed_inputs.proof_of_deposit {
        for pod in pod_link.data {
            // find the contract this is going to touch
            let contract_address = pod.contract_address;
            let amount = state[contract_address];
            if pod.method == "deposit" {
                let new_amount = pod.amount;
                amount = add_fuel(amount, new_amount);
            } else if pod.method == "withdraw" {
                let new_amount = pod.amount;
                amount = sub_fuel(amount, new_amount);
            } 
            state[contract_address] = amount;
            let amounts = #{};
            amounts[unyt_index] = pod.amount; 
            unyt_allocation.push(#{
                "receiver": pod.depositor_wallet_address,
                "amounts": amounts,
                "sources": [pod_link.link_hash]
            });
        }
    }
    return #{
        "unyt_allocation": unyt_allocation, 
        "state": state
    };
}

fn withdraw_logic(consumed_inputs, state, ea_id, unyt_index, executed_timestamp, cool_down_period, executor_pub_key, coupons) {
    let unyt_allocation = [];
    let rejected_links = [];
    let processed_coupons = #{};
    for allocation in consumed_inputs.withdrawer_allocations {
        if !(allocation.link_hash in coupons) {
            rejected_links.push(#{
                "hash": allocation.link_hash,
                "reason": "No coupon provided for this withdrawal"
            });
            continue;
        }
        let found_contract_address = false;
        let contract_address;
        for addresses in consumed_inputs.withdraw_contract_address {
            if addresses.link_hash == allocation.link_hash {
                found_contract_address = true;
                contract_address = addresses.data;
                break;
            }
        }
        let amount = state[contract_address];
        if type_of(amount) == "string" {
            let spent_amount = allocation.data.amount;
            amount = sub_fuel(amount, spent_amount[unyt_index]);
            // if is_greater_than_eq(amount, "0") {
                state[contract_address] = amount;
                unyt_allocation.push(#{
                    "receiver": executor_pub_key,
                    "amounts": spent_amount,
                    "sources": [allocation.link_hash]
                });
                processed_coupons[allocation.link_hash] = coupons[allocation.link_hash];
        } else {
            rejected_links.push(#{
                "hash": allocation.link_hash,
                "reason": "Failed to withdraw: contract_address does not have any balance"
            });
        }
    }
    return #{
        "unyt_allocation": unyt_allocation, 
        "state": state,
        "rejected_links": rejected_links,
        "processed_coupons": processed_coupons
    };
}

if type_of(inputs.previous_execution.data) == "map" {
    state = inputs.previous_execution.data.output.carryover;
}

let deposit_result = deposit_logic(consumed_inputs, state, unyt_index);
state = deposit_result.state;
let unyt_allocation = deposit_result.unyt_allocation;

let withdraw_result = withdraw_logic(
    consumed_inputs, state, ea_id, unyt_index,
    executed_timestamp, inputs.cool_down_period.data,
    executor_pub_key, inputs.coupons.data
);
state = withdraw_result.state;
for alloc in withdraw_result.unyt_allocation {
    unyt_allocation.push(alloc);
}

return #{
    "output": #{
        "unyt_allocation": unyt_allocation,
        "carryover": state,
        "computed_values": #{
            "coupons": withdraw_result.processed_coupons
        }
    },
    "rejected_links": withdraw_result.rejected_links
};