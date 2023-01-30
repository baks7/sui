// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import * as Yup from 'yup';

import { DEFAULT_GAS_BUDGET_FOR_STAKE } from '_redux/slices/sui-objects/Coin';
import { createTokenValidation } from '_src/shared/validation';

export function createValidationSchema(
    coinType: string,
    coinBalance: bigint,
    coinSymbol: string,
    gasBalance: bigint,
    decimals: number,
    gasDecimals: number,
    maxSuiSingleCoinBalance: bigint
) {
    return Yup.object({
        amount: createTokenValidation(
            coinType,
            coinBalance,
            coinSymbol,
            gasBalance,
            decimals,
            gasDecimals,
            DEFAULT_GAS_BUDGET_FOR_STAKE,
            maxSuiSingleCoinBalance
        ),
    });
}
