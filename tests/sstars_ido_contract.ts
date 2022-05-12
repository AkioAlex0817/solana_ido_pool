import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { SstarsIdoContract } from "../target/types/sstars_ido_contract";
import { Token, TOKEN_PROGRAM_ID } from "@solana/spl-token";

const utils = require("./utils");
import * as fs from "fs";
import * as assert from "assert";

const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const program = anchor.workspace
  .SstarsIdoContract as Program<SstarsIdoContract>;
const CONFIG_PDA_SEED = "config";

describe("sstars_ido_contract", () => {
  let stableCoinMintKeyPair: anchor.web3.Keypair;
  let stableCoinMintObject: Token;
  let stableCoinMintPubKey: anchor.web3.PublicKey;

  let client: anchor.web3.Keypair;
  let clientStableCoinWallet: anchor.web3.PublicKey;

  let service: anchor.web3.Keypair;
  let serviceStableCoinWallet: anchor.web3.PublicKey;

  // the program's ido_account account
  let idoPubKey: anchor.web3.PublicKey;
  let idoBump: number;

  let idoTimes;
  let idoName = "sstars_ido";

  it("Prepare", async () => {
    //Create StableCoin
    let keyPairFile = fs.readFileSync(
      "/home/alex/blockchain/cgc-solana-contracts/sstars_ido_contract/tests/keys/stablecoin.json",
      "utf-8"
    );
    let keyPairData = JSON.parse(keyPairFile);
    stableCoinMintKeyPair = anchor.web3.Keypair.fromSecretKey(
      new Uint8Array(keyPairData)
    );
    stableCoinMintObject = await utils.createMint(
      stableCoinMintKeyPair,
      provider,
      provider.wallet.publicKey,
      null,
      9,
      TOKEN_PROGRAM_ID
    );
    stableCoinMintPubKey = stableCoinMintObject.publicKey;
    console.log(stableCoinMintPubKey.toString());

    // Load Client
    let clientPairFile = fs.readFileSync(
      "/home/alex/blockchain/cgc-solana-contracts/sstars_ido_contract/tests/keys/client.json",
      "utf-8"
    );
    let clientPairData = JSON.parse(clientPairFile);
    client = anchor.web3.Keypair.fromSecretKey(new Uint8Array(clientPairData));

    // Airdrop 10 SOL to client
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        client.publicKey,
        10_000_000_000
      ),
      "confirmed"
    );

    // Load service
    let servicePairFile = fs.readFileSync(
      "/home/alex/blockchain/cgc-solana-contracts/sstars_ido_contract/tests/keys/service.json",
      "utf-8"
    );
    let servicePairData = JSON.parse(servicePairFile);
    service = anchor.web3.Keypair.fromSecretKey(
      new Uint8Array(servicePairData)
    );

    // create stable token wallet for client and service
    clientStableCoinWallet =
      await stableCoinMintObject.createAssociatedTokenAccount(client.publicKey);
    serviceStableCoinWallet =
      await stableCoinMintObject.createAssociatedTokenAccount(
        service.publicKey
      );

    // Airdrop stableCoin to client for test
    await utils.mintToAccount(
      provider,
      stableCoinMintPubKey,
      clientStableCoinWallet,
      1000_000_000_000
    );

    assert.strictEqual(
      await utils.getTokenBalance(provider, clientStableCoinWallet),
      1000_000_000_000
    );
    assert.strictEqual(
      await utils.getTokenBalance(provider, serviceStableCoinWallet),
      0
    );
  });
  it("Initialize", async () => {
    [idoPubKey, idoBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(idoName)],
      program.programId
    );

    idoTimes = new IdoTimes();
    const nowBn = new anchor.BN(Date.now() / 1000);
    idoTimes.startIdo = nowBn.add(new anchor.BN(5));
    idoTimes.endIdo = nowBn.add(new anchor.BN(3600));

    await program.methods
      .initialize(idoName, idoTimes, idoBump)
      .accounts({
        idoAuthority: provider.wallet.publicKey,
        idoAccount: idoPubKey,
        usdcMint: stableCoinMintPubKey,
        serviceVault: serviceStableCoinWallet,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      // @ts-ignore
      .signers([provider.wallet.payer])
      .rpc();
    const fetch = await program.account.idoAccount.fetch(idoPubKey);
    assert.strictEqual(
      fetch.usdcMint.toString(),
      stableCoinMintPubKey.toString()
    );
    assert.strictEqual(fetch.totalAmount.toNumber(), 0);
  });

  it("Stake Not started Error", async () => {
    const [user_stake, user_stake_bump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          client.publicKey.toBuffer(),
          Buffer.from(idoName),
          Buffer.from("user_stake"),
        ],
        program.programId
      );
    await assert.rejects(async () => {
      await program.methods
        .initUserStake()
        .accounts({
          userAuthority: client.publicKey,
          userStake: user_stake,
          idoAccount: idoPubKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([client])
        .rpc();
    });
  });
  it("Stake Low USDC Error", async () => {
    // Wait until the IDO has opened.
    if (Date.now() < idoTimes.startIdo.toNumber() * 1000) {
      await sleep(idoTimes.startIdo.toNumber() * 1000 - Date.now() + 4000);
    }
    const error_deposit = new anchor.BN(2000_000_000_000);
    const [user_stake, user_stake_bump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          client.publicKey.toBuffer(),
          Buffer.from(idoName),
          Buffer.from("user_stake"),
        ],
        program.programId
      );
    try {
      const instruction = await program.methods
        .initUserStake()
        .accounts({
          userAuthority: client.publicKey,
          userStake: user_stake,
          idoAccount: idoPubKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([client])
        .rpc();
      await assert.rejects(async () => {
        await program.methods
          .stake(error_deposit)
          .accounts({
            userAuthority: client.publicKey,
            idoAccount: idoPubKey,
            usdcMint: stableCoinMintPubKey,
            serviceVault: serviceStableCoinWallet,
            userStake: user_stake,
            userUsdc: clientStableCoinWallet,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([client])
          .rpc();
      });
    } catch (err) {
      console.log("This is the error message", err.toString());
    }
  });
  it("Stake Test", async () => {
    const deposit = new anchor.BN(500_000_000_000);
    const [user_stake, user_stake_bump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          client.publicKey.toBuffer(),
          Buffer.from(idoName),
          Buffer.from("user_stake"),
        ],
        program.programId
      );
    try {
      await program.methods
        .stake(deposit)
        .accounts({
          userAuthority: client.publicKey,
          idoAccount: idoPubKey,
          usdcMint: stableCoinMintPubKey,
          serviceVault: serviceStableCoinWallet,
          userStake: user_stake,
          userUsdc: clientStableCoinWallet,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([client])
        .rpc();
    } catch (err) {
      console.log("This is the error message", err.toString());
    }
    assert.strictEqual(
      await utils.getTokenBalance(provider, clientStableCoinWallet),
      500_000_000_000
    );
    assert.strictEqual(
      await utils.getTokenBalance(provider, serviceStableCoinWallet),
      500_000_000_000
    );
    const idoFetch = await program.account.idoAccount.fetch(idoPubKey);
    assert.strictEqual(idoFetch.totalAmount.toNumber(), 500_000_000_000);
    const userFetch = await program.account.userStake.fetch(user_stake);
    assert.strictEqual(userFetch.amount.toNumber(), 500_000_000_000);
  });

  function IdoTimes() {
    this.startIdo;
    this.endIdo;
  }

  // Our own sleep function.
  function sleep(ms) {
    console.log("Sleeping for", ms / 1000, "seconds");
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
});
