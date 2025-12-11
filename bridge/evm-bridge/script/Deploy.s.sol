// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import {EvmBridge} from "../src/EvmBridge.sol";

contract DeployScript is Script {
    using stdJson for string;

    function run() external {
        uint256 deployerPk = vm.envUint("ADMIN_PRIVATE_KEY");
        address relayer = vm.envAddress("RELAYER_ADDRESS");
        
        address admin = vm.addr(deployerPk);
        
        console.log("Deploying EvmBridge...");
        console.log("Admin:", admin);
        console.log("Relayer:", relayer);
        
        vm.startBroadcast(deployerPk);
        EvmBridge bridge = new EvmBridge(admin, relayer);
        vm.stopBroadcast();
        
        console.log("EvmBridge deployed at:", address(bridge));
        console.log("WrappedToken address:", address(bridge.wrappedToken()));
        
        // Alternative: Using stdJson library
        string memory json = "deployment";
        json.serialize("bridge", address(bridge));
        json.write("./deployment.json");
    }
}