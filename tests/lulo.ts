import * as anchor from "@project-serum/anchor";
import { Program, BN } from '@project-serum/anchor';
import { Lulo } from "../target/types/lulo";
import {
  PublicKey, Keypair, SystemProgram, Transaction, TransactionInstruction, LAMPORTS_PER_SOL,
  SYSVAR_RECENT_BLOCKHASHES_PUBKEY,
  SYSVAR_RENT_PUBKEY
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, NATIVE_MINT, ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount, createMint, mintToChecked, getAssociatedTokenAddress } from "@solana/spl-token";
import { assert } from "chai";

describe("lulo", () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(anchor.Provider.env());
  const program = anchor.workspace.Lulo as Program<Lulo>;

  // Constants
  const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
  const SYSVAR_RENT_PUBKEY = new PublicKey(
    'SysvarRent111111111111111111111111111111111',
  );

  // Params
  const amountDue = new anchor.BN(100);
  const fee = new anchor.BN(2);
  const feeScalar = new anchor.BN(1000); // 2.0%
  const payAmount = new anchor.BN(100);
  const dueDate = new anchor.BN(Math.floor(new Date('2022.08.10').getTime() / 1000))

  // Auths
  const luloAuth = Keypair.generate();
  const creatorAuth = Keypair.generate();
  const signerAuth = Keypair.generate();
  const approverAuth = Keypair.generate();
  const fakeApprover = Keypair.generate();

  // Accounts
  let contract = Keypair.generate();
  let contract2 = Keypair.generate();
  let mintAccount = null;
  let mintAccount2 = null;
  let source = null;
  let payMint = null;

  // PDAs
  let mint = null;
  let mint2 = null;
  let state = null;
  let vault = null;
  let approve = null;
  let fakeApprove = null;

  // Bumps
  let mintBump = null;
  let mint2Bump = null;
  let stateBump = null;
  let vaultBump = null;
  let approveBump = null;
  let fakeApproveBump = null;

  it("Initialize state", async () => {
    // Airdrop to creator auth
    const creatorAuthAirdrop = await provider.connection.requestAirdrop(creatorAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(creatorAuthAirdrop);
    // Airdrop to lulo auth
    const luloAuthAirdrop = await provider.connection.requestAirdrop(luloAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(luloAuthAirdrop);
    // Airdrop to signer
    const signerAuthAirdrop = await provider.connection.requestAirdrop(signerAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(signerAuthAirdrop);

    payMint = await createMint(
      provider.connection, // conneciton
      luloAuth, // fee payer
      luloAuth.publicKey, // mint authority
      luloAuth.publicKey, // freeze authority (you can use `null` to disable it. when you disable it, you can't turn it on again)
      6 // decimals
    );

    // Create token account for paying contract
    source = await createAssociatedTokenAccount(
      provider.connection, // connection
      luloAuth, // fee payer
      payMint, // mint
      signerAuth.publicKey // owner,
    );

    // Mint tokens to source
    let txhash = await mintToChecked(
      provider.connection, // connection
      luloAuth, // fee payer
      payMint, // mint
      source, // receiver (sholud be a token account)
      luloAuth, // mint authority
      1000, // amount. if your decimals is 8, you mint 10^8 for 1 token.
      6 // decimals
    );

    // Receivable mint PDA
    [mint, mintBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("mint")),
        contract.publicKey.toBuffer(),
      ],
      program.programId
    );
    // Receivable mint PDA
    [mint2, mint2Bump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("mint")),
        contract2.publicKey.toBuffer(),
      ],
      program.programId
    );
    // State PDA
    [state, stateBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("state")),
      ],
      program.programId
    );
    // Vault PDA
    [vault, vaultBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        payMint.toBuffer(),
      ],
      program.programId
    );
    // Approver PDA
    [approve, approveBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("approver")),
        signerAuth.publicKey.toBuffer(),
        approverAuth.publicKey.toBuffer(),
      ],
      program.programId
    );
    // Approver PDA
    [fakeApprove, fakeApproveBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("approver")),
        provider.wallet.publicKey.toBuffer(),
        fakeApprover.publicKey.toBuffer(),
      ],
      program.programId
    );
    // ATA Mint 1
    mintAccount = await getAssociatedTokenAddress(mint, creatorAuth.publicKey);
    // ATA Mint 2
    mintAccount2 = await getAssociatedTokenAddress(mint2, creatorAuth.publicKey);
  });

  it("Initialize program", async () => {
    await program.rpc.initialize(
      fee,
      feeScalar,
      {
        accounts: {
          signer: luloAuth.publicKey,
          state: state,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        },
        signers: [luloAuth]
      })
    let _state = await program.account.state.fetch(state);
    assert.ok(_state.fee.eq(fee))
    assert.ok(_state.feeScalar.eq(feeScalar))
    assert.ok(_state.admin.equals(luloAuth.publicKey))
  });

  it("Create vault", async () => {
    await program.rpc.createVault(
      {
        accounts: {
          signer: luloAuth.publicKey,
          vault: vault,
          mint: payMint,
          state: state,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        },
        signers: [luloAuth]
      })
    // Vault initialized
    let _vault = await provider.connection.getParsedAccountInfo(vault)
    assert.ok(_vault.value.data['parsed']['info']['mint'] == payMint)
    assert.ok(_vault.value.data['parsed']['info']['owner'] == vault)
  });

  it("Create contract", async () => {
    await program.rpc.create(
      amountDue,
      dueDate,
      {
        accounts: {
          signer: creatorAuth.publicKey,
          contract: contract.publicKey,
          mint: mint,
          mintAccount: mintAccount,
          payMint: payMint,
          vault: vault,
          recipient: signerAuth.publicKey,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        },
        signers: [creatorAuth, contract]
      })
    // Receivable initialized
    let _receivable = await program.account.contract.fetch(contract.publicKey);
    assert.ok(_receivable.amountDue.eq(amountDue))
    assert.ok(_receivable.mint.equals(mint))
    // Receivable is unsigned
    assert.ok(_receivable.approver.equals(PublicKey.default))
    // Recipient is correct
    assert.ok(_receivable.recipient.equals(signerAuth.publicKey))
    // Date
    assert.ok(_receivable.dueDate.eq(dueDate))
    // Mint account receives NFT
    let _balance = await provider.connection.getTokenAccountBalance(mintAccount)
    assert.ok(_balance.value.amount == '1')
  });

  it("Set fake approver", async () => {
    await program.rpc.setApprover(
      {
        accounts: {
          signer: provider.wallet.publicKey,
          delegate: fakeApprover.publicKey,
          approver: fakeApprove,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        },
        signers: []
      })
    // Fake approver created
    let _fakeApprove = await program.account.approver.fetch(fakeApprove);
    assert.ok(_fakeApprove.admin.equals(provider.wallet.publicKey))
    assert.ok(_fakeApprove.key.equals(fakeApprover.publicKey))
  });

  it("Set approver", async () => {
    await program.rpc.setApprover(
      {
        accounts: {
          signer: signerAuth.publicKey,
          delegate: approverAuth.publicKey,
          approver: approve,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        },
        signers: [signerAuth]
      })
    // Fake approver created
    let _approve = await program.account.approver.fetch(approve);
    assert.ok(_approve.admin.equals(signerAuth.publicKey))
    assert.ok(_approve.key.equals(approverAuth.publicKey))
  });

  it("Approve contract", async () => {
    await program.rpc.approve(
      {
        accounts: {
          signer: signerAuth.publicKey,
          contract: contract.publicKey,
          approver: fakeApprove,
        },
        signers: [signerAuth]
      })
    // Receivable signed
    let _receivable = await program.account.contract.fetch(contract.publicKey);
    assert.ok(_receivable.approver.equals(signerAuth.publicKey))
  });

  it("Pay contract", async () => {
    await program.rpc.pay(
      {
        accounts: {
          signer: signerAuth.publicKey,
          source: source,
          contract: contract.publicKey,
          vault: vault,
          payMint: payMint,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [signerAuth]
      })
    // Receivable paid
    let _receivable = await program.account.contract.fetch(contract.publicKey);
    assert.ok(_receivable.status.toNumber() == 2)
    // Vault has funds
    let _balance = await provider.connection.getTokenAccountBalance(vault);
    assert.ok(_balance.value.amount == _receivable.amountDue.toString())
  });

  it("Redeem Receivable", async () => {
    await program.rpc.redeem(
      {
        accounts: {
          signer: creatorAuth.publicKey,
          creator: creatorAuth.publicKey,
          contract: contract.publicKey,
          nftAccount: mintAccount,
          recipient: source,
          vault: vault,
          payMint: payMint,
          mint: mint,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        },
        signers: [creatorAuth]
      })
    // Funds sent to recipient
    let _balance = await provider.connection.getTokenAccountBalance(source)
    assert.ok(_balance.value.amount == '1000')
  });

  it("Create contract2", async () => {
    await program.rpc.create(
      amountDue,
      dueDate,
      {
        accounts: {
          signer: creatorAuth.publicKey,
          contract: contract2.publicKey,
          mint: mint2,
          mintAccount: mintAccount2,
          payMint: payMint,
          vault: vault,
          recipient: signerAuth.publicKey,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        },
        signers: [creatorAuth, contract2]
      })
    // Receivable initialized
    let _receivable = await program.account.contract.fetch(contract2.publicKey);
    assert.ok(_receivable.amountDue.eq(amountDue))
    assert.ok(_receivable.mint.equals(mint2))
    // Receivable is unsigned
    assert.ok(_receivable.approver.equals(PublicKey.default))
    // Recipient is correct
    assert.ok(_receivable.recipient.equals(signerAuth.publicKey))
    // Mint account receives NFT
    let _balance = await provider.connection.getTokenAccountBalance(mintAccount2)
    assert.ok(_balance.value.amount == '1')
  });

  it("Approve contract2", async () => {
    await program.rpc.approve(
      {
        accounts: {
          signer: approverAuth.publicKey,
          contract: contract2.publicKey,
          approver: approve,
        },
        signers: [approverAuth]
      })
    // Receivable signed
    let _receivable = await program.account.contract.fetch(contract2.publicKey);
    assert.ok(_receivable.approver.equals(approverAuth.publicKey))
  });
});
