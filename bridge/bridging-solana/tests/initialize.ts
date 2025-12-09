import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BridgingSolana } from "../target/types/bridging_solana";
import { Keypair, PublicKey, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  MINT_SIZE,
  createInitializeMintInstruction,
  getMinimumBalanceForRentExemptMint,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { expect } from "chai";

describe("bridging-solana", () => {
  // Configure the client
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.BridgingSolana as Program<BridgingSolana>;

  // Test accounts
  let admin: Keypair;
  let tokenMint: Keypair;
  let relayer: Keypair;
  let configPda: PublicKey;
  let vaultAuthorityPda: PublicKey;
  let tokenVaultPda: PublicKey;

  // Test parameters
  const destinationChainId = new anchor.BN(1);
  const destinationBridge = Buffer.from(
    "0x1234567890123456789012345678901234567890".slice(2),
    "hex"
  );
  const mintDecimals = 9;

  before(async () => {
    // Generate keypairs
    admin = Keypair.generate();
    tokenMint = Keypair.generate();
    relayer = Keypair.generate();

    // Airdrop SOL to admin for transaction fees
    const airdropSignature = await provider.connection.requestAirdrop(
      admin.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdropSignature);

    // Derive PDAs
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
  });

  describe("initialize", () => {
    it("Successfully initializes the bridge with valid parameters", async () => {
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

      const tx = await program.methods
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

      console.log("Initialize transaction signature:", tx);

      const configAccount = await program.account.bridgeConfig.fetch(configPda);

      expect(configAccount.admin.toString()).to.equal(
        admin.publicKey.toString()
      );
      expect(configAccount.tokenMint.toString()).to.equal(
        tokenMint.publicKey.toString()
      );
      expect(configAccount.nonce.toNumber()).to.equal(0);
      expect(configAccount.destinationChainId.toNumber()).to.equal(
        destinationChainId.toNumber()
      );
      expect(Buffer.from(configAccount.destinationBridge)).to.deep.equal(
        destinationBridge
      );
      expect(configAccount.relayerPubkey.toString()).to.equal(
        relayer.publicKey.toString()
      );
      expect(configAccount.vaultAuthorityBump).to.be.a("number");
      expect(configAccount.vaultAuthorityBump).to.be.greaterThan(0);
      expect(configAccount.vaultAuthorityBump).to.be.lessThan(256);

      console.log(" Initialize test passed!");
      console.log("   Admin:", configAccount.admin.toString());
      console.log("   Token Mint:", configAccount.tokenMint.toString());
      console.log("   Config PDA:", configPda.toString());
      console.log("   Vault Authority:", vaultAuthorityPda.toString());
      console.log("   Nonce:", configAccount.nonce.toString());
    });

    it("Fails to initialize bridge twice with same token mint", async () => {
      try {
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

        expect.fail("Should have thrown an error");
      } catch (err) {
        expect(err).to.be.instanceOf(Error);
        expect(err.toString()).to.include("already in use");
      }
    });

    it("Initializes bridge with different token mint", async () => {
      // Create a new token mint
      const newTokenMint = Keypair.generate();
      const mintRent = await getMinimumBalanceForRentExemptMint(
        provider.connection
      );

      // Create the new mint
      const createMintTx = new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: admin.publicKey,
          newAccountPubkey: newTokenMint.publicKey,
          space: MINT_SIZE,
          lamports: mintRent,
          programId: TOKEN_PROGRAM_ID,
        }),
        createInitializeMintInstruction(
          newTokenMint.publicKey,
          mintDecimals,
          admin.publicKey,
          null
        )
      );

      await provider.sendAndConfirm(createMintTx, [admin, newTokenMint]);

      // Derive new PDAs for the new mint
      const [newConfigPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("bridge"), newTokenMint.publicKey.toBuffer()],
        program.programId
      );

      const [newVaultAuthorityPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), newConfigPda.toBuffer()],
        program.programId
      );

      const newTokenVaultPda = await getAssociatedTokenAddress(
        newTokenMint.publicKey,
        newVaultAuthorityPda,
        true
      );

      // Initialize with new mint
      const tx = await program.methods
        .initialize(
          destinationChainId,
          Array.from(destinationBridge),
          relayer.publicKey
        )
        .accountsPartial({
          admin: admin.publicKey,
          tokenMint: newTokenMint.publicKey,
        })
        .signers([admin])
        .rpc();

      console.log("Second initialize transaction signature:", tx);

      // Verify the new config
      const newConfigAccount = await program.account.bridgeConfig.fetch(
        newConfigPda
      );

      expect(newConfigAccount.tokenMint.toString()).to.equal(
        newTokenMint.publicKey.toString()
      );
      expect(newConfigAccount.nonce.toNumber()).to.equal(0);
    });

    it("Initializes bridge with different destination chain ID", async () => {
      const newTokenMint = Keypair.generate();
      const mintRent = await getMinimumBalanceForRentExemptMint(
        provider.connection
      );

      const createMintTx = new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: admin.publicKey,
          newAccountPubkey: newTokenMint.publicKey,
          space: MINT_SIZE,
          lamports: mintRent,
          programId: TOKEN_PROGRAM_ID,
        }),
        createInitializeMintInstruction(
          newTokenMint.publicKey,
          mintDecimals,
          admin.publicKey,
          null
        )
      );

      await provider.sendAndConfirm(createMintTx, [admin, newTokenMint]);

      const [newConfigPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("bridge"), newTokenMint.publicKey.toBuffer()],
        program.programId
      );

      const [newVaultAuthorityPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), newConfigPda.toBuffer()],
        program.programId
      );

      const newTokenVaultPda = await getAssociatedTokenAddress(
        newTokenMint.publicKey,
        newVaultAuthorityPda,
        true
      );

      const polygonChainId = new anchor.BN(137);

      const tx = await program.methods
        .initialize(
          polygonChainId,
          Array.from(destinationBridge),
          relayer.publicKey
        )
        .accountsPartial({
          admin: admin.publicKey,
          tokenMint: newTokenMint.publicKey,
        })
        .signers([admin])
        .rpc();

      const configAccount = await program.account.bridgeConfig.fetch(
        newConfigPda
      );

      expect(configAccount.destinationChainId.toNumber()).to.equal(137);
      console.log(" Successfully initialized with different chain ID");
    });
  });
});
