import type { Program } from "@coral-xyz/anchor";
import * as anchor from "@coral-xyz/anchor";
import { BN } from "bn.js";
import { expect } from "chai";
import type { SwapExample } from "../target/types/swap_example";
import { createValues, mintingTokens, TestValues } from "./utils";

describe("Swapping tokens test", () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  anchor.setProvider(provider);

  const program = anchor.workspace.SwapExample as Program<SwapExample>;

  let values: TestValues;
  beforeEach(async () => {
    values = createValues();

    await program.methods
      .createAmm(values.id, values.fee)
      .accounts({ amm: values.ammKey, admin: values.admin.publicKey })
      .rpc();

    await mintingTokens({
      connection,
      creator: values.admin,
      mintAKeypair: values.mintAKeypair,
      mintBKeypair: values.mintBKeypair,
    });

    await program.methods
      .createPool()
      .accounts({
        amm: values.ammKey,
        pool: values.poolKey,
        poolAuthority: values.poolAuthority,
        mintLiquidity: values.mintLiquidity,
        mintA: values.mintAKeypair.publicKey,
        mintB: values.mintBKeypair.publicKey,
        poolAccountA: values.poolAccountA,
        poolAccountB: values.poolAccountB,
      })
      .rpc();

    await program.methods
      .depositLiquidity(values.depositAmountA, values.depositAmountB)
      .accounts({
        pool: values.poolKey,
        poolAuthority: values.poolAuthority,
        depositor: values.admin.publicKey,
        mintLiquidity: values.mintLiquidity,
        mintA: values.mintAKeypair.publicKey,
        mintB: values.mintBKeypair.publicKey,
        poolAccountA: values.poolAccountA,
        poolAccountB: values.poolAccountB,
        depositorAccountLiquidity: values.liquidityAccount,
        depositorAccountA: values.holderAccountA,
        depositorAccountB: values.holderAccountB,
      })
      .signers([values.admin])
      .rpc({ skipPreflight: true });
  });

  it("Swap from a to b", async () => {
    const input = new BN(10 ** 6);
    await program.methods
      .swapTokens(true, input, new BN(100))
      .accounts({
        amm: values.ammKey.toBase58(),
        pool: values.poolKey,
        poolAuthority: values.poolAuthority,
        mintLiquidity: values.mintLiquidity,
        mintA: values.mintAKeypair.publicKey,
        mintB: values.mintBKeypair.publicKey,
        poolAccountA: values.poolAccountA,
        poolAccountB: values.poolAccountB,
        trader: values.admin.publicKey,
        traderAccountA: values.holderAccountA,
        traderAccountB: values.holderAccountB,
      })
      .signers([values.admin])
      .rpc();

    const traderAccountA = await connection.getTokenAccountBalance(
      values.holderAccountA
    );
    const traderAccountB = await connection.getTokenAccountBalance(
      values.holderAccountB
    );
    expect(traderAccountA.value.amount).to.equal(
      values.defaultSupply.sub(values.depositAmountA).sub(input).toString()
    );
    expect(Number(traderAccountB.value.amount)).to.be.greaterThan(
      values.defaultSupply.sub(values.depositAmountB).toNumber()
    );
    expect(Number(traderAccountB.value.amount)).to.be.lessThan(
      values.defaultSupply.sub(values.depositAmountB).add(input).toNumber()
    );
  });
});
