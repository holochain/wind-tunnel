let unyt_index = inputs.unyt_index.data;

let amount = "0";
if type_of(inputs.previous_execution.data) == "map" {
    amount = inputs.previous_execution.data.output.credit_limit[unyt_index];
}

// use the output of this agreement to update it based on the input
for pod in consumed_inputs.proof_of_deposit {
    for proof in pod.data {
        if proof.method == "deposit" {
            let new_amount = proof.amount;
            amount = add_fuel(amount, new_amount);
        } 
        else if proof.method == "withdraw" {
            let new_amount = proof.amount;
            amount = sub_fuel(amount, new_amount);
        } 
        else {
            error("Invalid method");
        }
    }
}

// INDEX of the external unyt that the credit limit is adjusted for
let credit_limit = #{};
credit_limit[unyt_index] = amount;

return #{
    "output": #{
        "credit_limit": credit_limit
    }
};
