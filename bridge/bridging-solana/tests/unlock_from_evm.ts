import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BridgingSolana } from "../target/types/bridging_solana";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
  createAssociatedTokenAccountInstruction,
  createInitializeMintInstruction,
  createMint,
  createMintToInstruction,
  getAccount,
  getAssociatedTokenAddress,
  getMinimumBalanceForRentExemptAccount,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";

describe("unlock from evm", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.BridgingSolana as Program<BridgingSolana>;

  let admin: Keypair;
  let tokenMint: Keypair;
  let relayer: Keypair;
  let unauthorizedRelayer: Keypair;
  let recipient: Keypair;
  let configPda: PublicKey;
  let vaultAuthorityPda: PublicKey;
  let tokenVaultPda: PublicKey;
  let recipientTokenAccount: PublicKey;

  const destinationChainId = new anchor.BN(1);
  const destinationBridge = Buffer.from(
    "0x1234567890123456789012345678901234567890".slice(2),
    "hex"
  );
  const mintDecimals = 9;
  const unlockAmount = new anchor.BN(500 * 10 ** mintDecimals);
  const srcChainId = new anchor.BN(1);
  const nonce = new anchor.BN(42);

  before(async () => {
    admin = Keypair.generate();
    tokenMint = Keypair.generate();
    relayer = Keypair.generate();
    unauthorizedRelayer = Keypair.generate();
    recipient = Keypair.generate();

    // airdrop
    const adminAirdrop = await provider.connection.requestAirdrop(
      admin.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(adminAirdrop);

    const relayerAirdrop = await provider.connection.requestAirdrop(
      relayer.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(relayerAirdrop);

    const recipientAirdrop = await provider.connection.requestAirdrop(
      recipient.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(recipientAirdrop);

    const unauthorizedRelayerAirdrop = await provider.connection.requestAirdrop(
      unauthorizedRelayer.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(unauthorizedRelayerAirdrop);

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

    recipientTokenAccount = await getAssociatedTokenAddress(
      tokenMint.publicKey,
      recipient.publicKey
    );

    const mintRent = await getMinimumBalanceForRentExemptAccount(
      provider.connection
    );

    const createMintTx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        lamports: mintRent,
        newAccountPubkey: tokenMint.publicKey,
        space: MINT_SIZE,
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

    // init bridge
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

    // fund
    const vaultMintTx = new anchor.web3.Transaction().add(
      createMintToInstruction(
        tokenMint.publicKey,
        tokenVaultPda,
        admin.publicKey,
        unlockAmount.toNumber() * 2,
        [],
        TOKEN_PROGRAM_ID
      )
    );

    await provider.sendAndConfirm(vaultMintTx, [admin]);

    const recipientAccountInfo = await provider.connection.getAccountInfo(
      recipientTokenAccount
    );

    if (!recipientAccountInfo) {
      const createAccountTx = new anchor.web3.Transaction().add(
        createAssociatedTokenAccountInstruction(
          admin.publicKey,
          recipientTokenAccount,
          recipient.publicKey,
          tokenMint.publicKey
        )
      );

      await provider.sendAndConfirm(createAccountTx, [admin]);
    }
  });

  it("Successfull unlock tokens from evm bridge", async () => {
    const processedMessagePda = PublicKey.findProgramAddressSync(
      [
        Buffer.from("processed"),
        srcChainId.toArrayLike(Buffer, "le", 8),
        nonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    )[0];

    const vaultBefore = await getAccount(provider.connection, tokenVaultPda);
    const recipientBefore = await getAccount(
      provider.connection,
      recipientTokenAccount
    );
    const vaultBalanceBefore = Number(vaultBefore.amount);
    const recipientBalanceBefore = Number(recipientBefore?.amount || 0);

    // console.log(`Vault balance before: ${vaultBalanceBefore}`);
    // console.log(`Recipient balance before: ${recipientBalanceBefore}`);

    const tx = await program.methods
      .unlockFromEvm(srcChainId, nonce, unlockAmount)
      .accounts({
        relayer: relayer.publicKey,
        config: configPda,
        processedMessage: processedMessagePda,
        vaultAuthority: vaultAuthorityPda,
        tokenVault: tokenVaultPda,
        recipientTokenAccount: recipientTokenAccount,
        recipient: recipient.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([relayer])
      .rpc();

    // console.log(`Unlock transaction: ${tx}`);

    const processedMessage = await program.account.processedMessage.fetch(
      processedMessagePda
    );
    expect(processedMessage.executed).to.be.true;

    const vaultAfter = await getAccount(provider.connection, tokenVaultPda);
    const recipientAfter = await getAccount(
      provider.connection,
      recipientTokenAccount
    );
    const vaultBalanceAfter = Number(vaultAfter.amount);
    const recipientBalanceAfter = Number(recipientAfter.amount);

    // console.log(`Vault balance after: ${vaultBalanceAfter}`);
    // console.log(`Recipient balance after: ${recipientBalanceAfter}`);

    expect(vaultBalanceAfter).to.equal(
      vaultBalanceBefore - unlockAmount.toNumber()
    );
    expect(recipientBalanceAfter).to.equal(
      recipientBalanceBefore + unlockAmount.toNumber()
    );
  });

  it("Fails to unlock with the same nonce twice", async () => {
    const processedMessagePda = PublicKey.findProgramAddressSync(
      [
        Buffer.from("processed"),
        srcChainId.toArrayLike(Buffer, "le", 8),
        nonce.toArrayLike(Buffer, "le", 8)
      ], program.programId
    )[0];

    try {
      await program.methods.unlockFromEvm(srcChainId, nonce, unlockAmount).accounts({
        relayer: relayer.publicKey,
        config: configPda,
        processedMessage: processedMessagePda,
        vaultAuthority: vaultAuthorityPda,
        tokenVault: tokenVaultPda,
        recipientTokenAccount: recipientTokenAccount,
        recipient: recipient.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      }).signers([relayer]).rpc();

      expect.fail("Should have thrown an error");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
      expect(err.toString()).to.include("AlreadyProcessed");
    }
  })

  it("Fails when unauthorized relayer tries to unlock", async () => {
    const newNonce = new anchor.BN(100);
    const processedMessagePda = PublicKey.findProgramAddressSync(
      [
        Buffer.from("processed"),
        srcChainId.toArrayLike(Buffer, "le", 8),
        newNonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    )[0];

    try {
      await program.methods
        .unlockFromEvm(srcChainId, newNonce, unlockAmount)
        .accounts({
          relayer: unauthorizedRelayer.publicKey,
          config: configPda,
          processedMessage: processedMessagePda,
          vaultAuthority: vaultAuthorityPda,
          tokenVault: tokenVaultPda,
          recipientTokenAccount: recipientTokenAccount,
          recipient: recipient.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([unauthorizedRelayer])
        .rpc();

      expect.fail("Should have thrown an error");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
      expect(err.toString()).to.include("Unauthorized");
    }
  });

  it("Fails to unlock when bridge is paused", async () => {
    // Pause the bridge first
    await program.methods
      .pauseBridge()
      .accounts({
        admin: admin.publicKey,
        config: configPda,
      } as any)
      .signers([admin])
      .rpc();

    const pausedNonce = new anchor.BN(200);
    const processedMessagePda = PublicKey.findProgramAddressSync(
      [
        Buffer.from("processed"),
        srcChainId.toArrayLike(Buffer, "le", 8),
        pausedNonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    )[0];

    try {
      await program.methods
        .unlockFromEvm(srcChainId, pausedNonce, unlockAmount)
        .accounts({
          relayer: relayer.publicKey,
          config: configPda,
          processedMessage: processedMessagePda,
          vaultAuthority: vaultAuthorityPda,
          tokenVault: tokenVaultPda,
          recipientTokenAccount: recipientTokenAccount,
          recipient: recipient.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([relayer])
        .rpc();

      expect.fail("Should have thrown an error");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
      expect(err.toString()).to.include("BridgePaused");
    }

    // Resume bridge for other tests
    await program.methods
      .resumeBridge()
      .accounts({
        admin: admin.publicKey,
        config: configPda,
      } as any)
      .signers([admin])
      .rpc();
  });

});

