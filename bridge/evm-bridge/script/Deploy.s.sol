// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script} from "forge-std/Script.sol";
import {EvmBridge} from "../src/EvmBridge.sol";

contract CounterScript is Script {

  function run() external {
        uint256 deployerPk = vm.envUint("ADMIN_PRIVATE_KEY");
        address relayer = vm.envAddress("RELAYER_ADDRESS");

        address admin = vm.addr(deployerPk);

        vm.startBroadcast(deployerPk);
        EvmBridge bridge = new EvmBridge(admin, relayer);
        vm.stopBroadcast();

    }

}
