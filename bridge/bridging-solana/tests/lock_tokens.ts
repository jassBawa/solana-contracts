import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BridgingSolana } from "../target/types/bridging_solana";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  MINT_SIZE,
  createInitializeMintInstruction,
  getMinimumBalanceForRentExemptMint,
  getAssociatedTokenAddress,
  createMintToInstruction,
  getAccount,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import { expect } from "chai";

describe("lock_tokens", () => {
  // Configure the client
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.BridgingSolana as Program<BridgingSolana>;

  // Test accounts
  let admin: Keypair;
  let tokenMint: Keypair;
  let relayer: Keypair;
  let user: Keypair;
  let configPda: PublicKey;
  let vaultAuthorityPda: PublicKey;
  let tokenVaultPda: PublicKey;
  let userTokenAccount: PublicKey;

  // Test parameters
  const destinationChainId = new anchor.BN(1);
  const destinationBridge = Buffer.from(
    "0x1234567890123456789012345678901234567890".slice(2),
    "hex"
  );
  const mintDecimals = 9;
  const lockAmount = new anchor.BN(1000 * 10 ** mintDecimals);
  before(async () => {
    // Generate keypairs
    admin = Keypair.generate();
    tokenMint = Keypair.generate();
    relayer = Keypair.generate();
    user = Keypair.generate();

    // Airdrop SOL to admin and user
    const adminAirdrop = await provider.connection.requestAirdrop(
      admin.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(adminAirdrop);

    const userAirdrop = await provider.connection.requestAirdrop(
      user.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(userAirdrop);

    [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bridge"), tokenMint.publicKey.toBuffer()],
      program.programId
    );

    [vaultAuthorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), configPda.toBuffer()],
      program.programId
    );

    tokenVaultPda = await getAssociatedTokenAddress(
      tokenMint.publicKey,
      vaultAuthorityPda,
      true
    );

    userTokenAccount = await getAssociatedTokenAddress(
      tokenMint.publicKey,
      user.publicKey
    );

    const mintRent = await getMinimumBalanceForRentExemptMint(
      provider.connection
    );

    const createMintTx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: tokenMint.publicKey,
        space: MINT_SIZE,
        lamports: mintRent,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        tokenMint.publicKey,
        mintDecimals,
        admin.publicKey,
        null
      )
    );

    await provider.sendAndConfirm(createMintTx, [admin, tokenMint]);

    await program.methods
      .initialize(
        destinationChainId,
        Array.from(destinationBridge),
        relayer.publicKey
      )
      .accountsPartial({
        admin: admin.publicKey,
        tokenMint: tokenMint.publicKey,
      })
      .signers([admin])
      .rpc();

    try {
      await getAccount(provider.connection, userTokenAccount);
    } catch (error) {
      const createAtaTx = new anchor.web3.Transaction().add(
        createAssociatedTokenAccountInstruction(
          admin.publicKey,
          userTokenAccount,
          user.publicKey,
          tokenMint.publicKey
        )
      );
      await provider.sendAndConfirm(createAtaTx, [admin]);
    }

    const totalTokensNeeded = new anchor.BN(5000 * 10 ** mintDecimals);
    const mintToTx = new anchor.web3.Transaction().add(
      createMintToInstruction(
        tokenMint.publicKey,
        userTokenAccount,
        admin.publicKey,
        totalTokensNeeded.toNumber(),
        []
      )
    );
    await provider.sendAndConfirm(mintToTx, [admin]);
  });

  it("Successfully locks tokens 5 times and verifies nonce increments", async () => {
    console.log("Config PDA:", configPda.toString());
    console.log("User:", user.publicKey.toString());
    console.log("Token Mint:", tokenMint.publicKey.toString());
    console.log("Lock Amount per operation:", lockAmount.toString(), "\n");

    // Fetch initial config state
    let configBefore = await program.account.bridgeConfig.fetch(configPda);
    let initialNonce = configBefore.nonce.toNumber();
    let expectedNonce = initialNonce;


    // Get initial balances
    const vaultBefore = await getAccount(provider.connection, tokenVaultPda);
    const userAccountBefore = await getAccount(
      provider.connection,
      userTokenAccount
    );
    const initialVaultBalance = Number(vaultBefore.amount);
    const initialUserBalance = Number(userAccountBefore.amount);
    let vaultBalanceBefore = initialVaultBalance;
    // let userBalanceBefore = initialUserBalance;

    const lockRecords: PublicKey[] = [];
    const destinationAddresses: Buffer[] = [];

    // Perform 5 lock operations
    for (let i = 0; i < 5; i++) {

      const destinationAddress = Buffer.from(
        `0x${(i + 1).toString().padStart(2, "0").repeat(20)}`.slice(2),
        "hex"
      );
      destinationAddresses.push(destinationAddress);

      const nonceBuffer = Buffer.allocUnsafe(8);
      nonceBuffer.writeBigUInt64LE(BigInt(expectedNonce), 0);
      const [lockRecordPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("lock"), configPda.toBuffer(), nonceBuffer],
        program.programId
      );
      lockRecords.push(lockRecordPda);

      const userAccount = await getAccount(
        provider.connection,
        userTokenAccount
      );
      const userBalance = Number(userAccount.amount);

      console.log(`  Nonce: ${expectedNonce}`);
      console.log(`  User balance before: ${userBalance}`);
      console.log(`  Lock Record PDA: ${lockRecordPda.toString()}`);

      // Lock tokens
      const tx = await program.methods
        .lockTokens(lockAmount, Array.from(destinationAddress))
        .accounts({
          user: user.publicKey,
          userTokenAccount: userTokenAccount,
          config: configPda,
          vaultAuthority: vaultAuthorityPda,
          tokenVault: tokenVaultPda,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        } as any)
        .signers([user])
        .rpc();

      console.log(`  Transaction: ${tx}`);

      const lockRecord = await program.account.lockRecord.fetch(lockRecordPda);
      expect(lockRecord.config.toString()).to.equal(configPda.toString());
      expect(lockRecord.nonce.toNumber()).to.equal(expectedNonce);
      expect(lockRecord.user.toString()).to.equal(user.publicKey.toString());
      expect(lockRecord.amount.toNumber()).to.equal(lockAmount.toNumber());
      expect(Buffer.from(lockRecord.destinationAddress)).to.deep.equal(
        destinationAddress
      );

      const configAfter = await program.account.bridgeConfig.fetch(configPda);
      expectedNonce = expectedNonce + 1;
      expect(configAfter.nonce.toNumber()).to.equal(expectedNonce);

      const userAccountAfter = await getAccount(
        provider.connection,
        userTokenAccount
      );
      const vaultAfter = await getAccount(provider.connection, tokenVaultPda);
      const userBalanceAfter = Number(userAccountAfter.amount);
      const vaultBalanceAfter = Number(vaultAfter.amount);

      console.log(`  User balance after: ${userBalanceAfter}`);
      console.log(`  Vault balance after: ${vaultBalanceAfter}`);
      console.log(`  New nonce: ${configAfter.nonce.toNumber()}`);
      console.log(`   Lock ${i + 1} successful\n`);

      expect(userBalanceAfter).to.equal(userBalance - lockAmount.toNumber());
      expect(vaultBalanceAfter).to.equal(
        vaultBalanceBefore + lockAmount.toNumber()
      );

      vaultBalanceBefore = vaultBalanceAfter;
    }

    const finalConfig = await program.account.bridgeConfig.fetch(configPda);
    const finalUserAccount = await getAccount(
      provider.connection,
      userTokenAccount
    );
    const finalVaultAccount = await getAccount(
      provider.connection,
      tokenVaultPda
    );

    console.log(`Final nonce: ${finalConfig.nonce.toNumber()}`);
    console.log(`Expected nonce: ${initialNonce + 5}`);
    console.log(`Final user balance: ${Number(finalUserAccount.amount)}`);
    console.log(`Final vault balance: ${Number(finalVaultAccount.amount)}`);
    console.log(`Total locked: ${5 * lockAmount.toNumber()}`);

    // Verify final state
    expect(finalConfig.nonce.toNumber()).to.equal(initialNonce + 5);
    expect(Number(finalUserAccount.amount)).to.equal(
      initialUserBalance - 5 * lockAmount.toNumber()
    );
    expect(Number(finalVaultAccount.amount)).to.equal(
      initialVaultBalance + 5 * lockAmount.toNumber()
    );

    for (let i = 0; i < 5; i++) {
      const lockRecord = await program.account.lockRecord.fetch(lockRecords[i]);
      expect(lockRecord.nonce.toNumber()).to.equal(initialNonce + i);
      expect(lockRecord.amount.toNumber()).to.equal(lockAmount.toNumber());
    }

    console.log("\n All 5 lock operations completed successfully!");
  });

  it("Fails to lock tokens with zero amount", async () => {
    const destinationAddress = Buffer.from(
      "0x1111111111111111111111111111111111111111".slice(2),
      "hex"
    );

    try {
      await program.methods
        .lockTokens(new anchor.BN(0), Array.from(destinationAddress))
        .accounts({
          user: user.publicKey,
          userTokenAccount: userTokenAccount,
          config: configPda,
          vaultAuthority: vaultAuthorityPda,
          tokenVault: tokenVaultPda,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        } as any)
        .signers([user])
        .rpc();

      expect.fail("Should have thrown an error");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
      expect(err.toString()).to.include("InvalidAmount");
    }
  });

  it("Fails to lock more tokens than user has", async () => {
    // Get user's current balance
    const userAccount = await getAccount(provider.connection, userTokenAccount);
    const userBalance = Number(userAccount.amount);

    // Try to lock more than user has
    const excessiveAmount = new anchor.BN(userBalance + 1000);
    const destinationAddress = Buffer.from(
      "0x3333333333333333333333333333333333333333".slice(2),
      "hex"
    );

    try {
      await program.methods
        .lockTokens(excessiveAmount, Array.from(destinationAddress))
        .accounts({
          user: user.publicKey,
          userTokenAccount: userTokenAccount,
          config: configPda,
          vaultAuthority: vaultAuthorityPda,
          tokenVault: tokenVaultPda,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        } as any)
        .signers([user])
        .rpc();

      expect.fail("Should have thrown an error");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
      // Should fail due to insufficient balance
      expect(err.toString()).to.include("insufficient funds");
    }
  });
});
