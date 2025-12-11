import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BridgingSolana } from "../target/types/bridging_solana";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
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

describe("pause and resume", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.BridgingSolana as Program<BridgingSolana>;

  let admin: Keypair;
  let unauthorizedAdmin: Keypair;
  let tokenMint: Keypair;
  let relayer: Keypair;
  let user: Keypair;
  let configPda: PublicKey;
  let vaultAuthorityPda: PublicKey;
  let tokenVaultPda: PublicKey;
  let userTokenAccount: PublicKey;

  const destinationChainId = new anchor.BN(1);
  const destinationBridge = Buffer.from(
    "0x1234567890123456789012345678901234567890".slice(2),
    "hex"
  );
  const mintDecimals = 9;
  const lockAmount = new anchor.BN(1000 * 10 ** mintDecimals);
  const destinationAddress = Buffer.from(
    "0x1111111111111111111111111111111111111111".slice(2),
    "hex"
  );

  before(async () => {
    admin = Keypair.generate();
    unauthorizedAdmin = Keypair.generate();
    tokenMint = Keypair.generate();
    relayer = Keypair.generate();
    user = Keypair.generate();

    // Airdrop SOL
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

    // Initialize bridge
    await program.methods
      .initialize(destinationChainId, Array.from(destinationBridge), relayer.publicKey)
      .accountsPartial({
        admin: admin.publicKey,
        tokenMint: tokenMint.publicKey,
      })
      .signers([admin])
      .rpc();

    // Create and fund user token account
    const userAccountInfo = await provider.connection.getAccountInfo(
      userTokenAccount
    );
    if (!userAccountInfo) {
      const createAccountTx = new anchor.web3.Transaction().add(
        createAssociatedTokenAccountInstruction(
          admin.publicKey,
          userTokenAccount,
          user.publicKey,
          tokenMint.publicKey
        )
      );
      await provider.sendAndConfirm(createAccountTx, [admin]);
    }

    const mintToUserTx = new anchor.web3.Transaction().add(
      createMintToInstruction(
        tokenMint.publicKey,
        userTokenAccount,
        admin.publicKey,
        lockAmount.toNumber() * 2,
        [],
        TOKEN_PROGRAM_ID
      )
    );

    await provider.sendAndConfirm(mintToUserTx, [admin]);
  });

  describe("Pause functionality", () => {
    it("Admin can pause the bridge", async () => {
      const configBefore = await program.account.bridgeConfig.fetch(configPda);
      expect(configBefore.paused).to.be.false;

      const tx = await program.methods
        .pauseBridge()
        .accounts({
          admin: admin.publicKey,
          config: configPda,
        } as any)
        .signers([admin])
        .rpc();


      const configAfter = await program.account.bridgeConfig.fetch(configPda);
      expect(configAfter.paused).to.be.true;
    });

    it("Fails to pause when already paused", async () => {
      try {
        await program.methods
          .pauseBridge()
          .accounts({
            admin: admin.publicKey,
            config: configPda,
          } as any)
          .signers([admin])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err) {
        expect(err).to.be.instanceOf(Error);
        expect(err.toString()).to.include("AlreadyPaused");
      }
    });

    it("Fails when non-admin tries to pause", async () => {
      // First resume so we can test pause again
      await program.methods
        .resumeBridge()
        .accounts({
          admin: admin.publicKey,
          config: configPda,
        } as any)
        .signers([admin])
        .rpc();

      try {
        await program.methods
          .pauseBridge()
          .accounts({
            admin: unauthorizedAdmin.publicKey,
            config: configPda,
          } as any)
          .signers([unauthorizedAdmin])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err) {
        expect(err).to.be.instanceOf(Error);
        expect(err.toString()).to.include("UnauthorizedAdmin");
      }
    });

    it("Prevents lock_tokens when paused", async () => {
      const config = await program.account.bridgeConfig.fetch(configPda);
      if (!config.paused) {
        await program.methods
          .pauseBridge()
          .accounts({
            admin: admin.publicKey,
            config: configPda,
          } as any)
          .signers([admin])
          .rpc();
      }

      try {
        await program.methods
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

        expect.fail("Should have thrown an error");
      } catch (err) {
        expect(err).to.be.instanceOf(Error);
        expect(err.toString()).to.include("BridgePaused");
      }
    });
  });

  describe("Resume functionality", () => {
    it("Admin can resume the bridge", async () => {
      const configBefore = await program.account.bridgeConfig.fetch(configPda);
      if (!configBefore.paused) {
        await program.methods
          .pauseBridge()
          .accounts({
            admin: admin.publicKey,
            config: configPda,
          } as any)
          .signers([admin])
          .rpc();
      }

      const tx = await program.methods
        .resumeBridge()
        .accounts({
          admin: admin.publicKey,
          config: configPda,
        } as any)
        .signers([admin])
        .rpc();

      const configAfter = await program.account.bridgeConfig.fetch(configPda);
      expect(configAfter.paused).to.be.false;
    });

    it("Fails to resume when not paused", async () => {
      try {
        await program.methods
          .resumeBridge()
          .accounts({
            admin: admin.publicKey,
            config: configPda,
          } as any)
          .signers([admin])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err) {
        expect(err).to.be.instanceOf(Error);
        expect(err.toString()).to.include("NotPaused");
      }
    });

    it("Fails when non-admin tries to resume", async () => {
      await program.methods
        .pauseBridge()
        .accounts({
          admin: admin.publicKey,
          config: configPda,
        } as any)
        .signers([admin])
        .rpc();

      try {
        await program.methods
          .resumeBridge()
          .accounts({
            admin: unauthorizedAdmin.publicKey,
            config: configPda,
          } as any)
          .signers([unauthorizedAdmin])
          .rpc();

        expect.fail("Should have thrown an error");
      } catch (err) {
        expect(err).to.be.instanceOf(Error);
        expect(err.toString()).to.include("UnauthorizedAdmin");
      }

      await program.methods
        .resumeBridge()
        .accounts({
          admin: admin.publicKey,
          config: configPda,
        } as any)
        .signers([admin])
        .rpc();
    });

    it("Allows lock_tokens after resume", async () => {
      const config = await program.account.bridgeConfig.fetch(configPda);
      if (config.paused) {
        await program.methods
          .resumeBridge()
          .accounts({
            admin: admin.publicKey,
            config: configPda,
          } as any)
          .signers([admin])
          .rpc();
      }

      const configBefore = await program.account.bridgeConfig.fetch(configPda);
      const initialNonce = configBefore.nonce.toNumber();

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


      const configAfter = await program.account.bridgeConfig.fetch(configPda);
      expect(configAfter.nonce.toNumber()).to.equal(initialNonce + 1);
    });
  });
});