# File Structure

All source code files in the repository.

```
.
├── .changeset
│   └── config.json
├── .github
│   └── workflows
│       └── release.yml
├── examples
│   ├── browser-sdk-demo-app
│   │   ├── src
│   │   │   ├── utils
│   │   │   │   └── balance.ts
│   │   │   ├── App.tsx
│   │   │   ├── index.css
│   │   │   ├── main.tsx
│   │   │   └── vite-env.d.ts
│   │   ├── index.html
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── tsconfig.node.json
│   │   └── vite.config.ts
│   ├── client-demo-app
│   │   ├── src
│   │   │   ├── index.ts
│   │   │   └── multi-auth-demo.ts
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── react-native-sdk-demo-app
│   │   ├── app
│   │   │   ├── _layout.tsx
│   │   │   ├── auth-callback.tsx
│   │   │   ├── index.tsx
│   │   │   ├── providers.tsx
│   │   │   └── wallet.tsx
│   │   ├── hooks
│   │   │   └── useBalance.ts
│   │   ├── .eslintrc.js
│   │   ├── app.json
│   │   ├── babel.config.js
│   │   ├── metro.config.js
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── react-sdk-demo-app
│   │   ├── src
│   │   │   ├── components
│   │   │   │   └── DebugConsole.tsx
│   │   │   ├── contexts
│   │   │   │   └── DebugContext.tsx
│   │   │   ├── hooks
│   │   │   │   └── useBalance.ts
│   │   │   ├── Actions.css
│   │   │   ├── Actions.tsx
│   │   │   ├── App.tsx
│   │   │   ├── AuthCallback.css
│   │   │   ├── AuthCallback.tsx
│   │   │   ├── SDKActions.tsx
│   │   │   ├── index.css
│   │   │   ├── main.tsx
│   │   │   └── vite-env.d.ts
│   │   ├── index.html
│   │   ├── package.json
│   │   ├── tsconfig.app.json
│   │   ├── tsconfig.json
│   │   ├── tsconfig.node.json
│   │   └── vite.config.ts
│   ├── server-sdk-examples
│   │   ├── src
│   │   │   ├── list-wallets.ts
│   │   │   ├── server-sdk-demo.ts
│   │   │   └── sign-message.ts
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── with-modal
│   │   ├── src
│   │   │   ├── App.tsx
│   │   │   ├── ConnectExample.tsx
│   │   │   ├── Sidebar.tsx
│   │   │   └── main.tsx
│   │   ├── index.html
│   │   ├── package.json
│   │   ├── tsconfig.app.json
│   │   ├── tsconfig.json
│   │   ├── tsconfig.node.json
│   │   └── vite.config.ts
│   ├── with-nextjs
│   │   ├── src
│   │   │   ├── app
│   │   │   │   ├── globals.css
│   │   │   │   ├── layout.tsx
│   │   │   │   └── page.tsx
│   │   │   └── components
│   │   │       ├── ClientProvider.tsx
│   │   │       ├── ServerSafeLayout.tsx
│   │   │       └── WalletInterface.tsx
│   │   ├── eslint.config.mjs
│   │   ├── next.config.ts
│   │   ├── package.json
│   │   ├── postcss.config.mjs
│   │   └── tsconfig.json
│   └── with-wagmi
│       ├── public
│       │   ├── auth-callback.html
│       │   └── index.html
│       ├── src
│       │   ├── App.tsx
│       │   ├── WalletDemo.tsx
│       │   ├── index.css
│       │   ├── index.tsx
│       │   ├── phantom-connector.ts
│       │   └── wagmi.ts
│       ├── package.json
│       └── tsconfig.json
├── packages
│   ├── api-key-stamper
│   │   ├── src
│   │   │   ├── index.test.ts
│   │   │   └── index.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── base64url
│   │   ├── src
│   │   │   ├── index.test.ts
│   │   │   └── index.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── browser-injected-sdk
│   │   ├── src
│   │   │   ├── auto-confirm
│   │   │   │   ├── autoConfirmDisable.test.ts
│   │   │   │   ├── autoConfirmDisable.ts
│   │   │   │   ├── autoConfirmEnable.test.ts
│   │   │   │   ├── autoConfirmEnable.ts
│   │   │   │   ├── autoConfirmStatus.test.ts
│   │   │   │   ├── autoConfirmStatus.ts
│   │   │   │   ├── autoConfirmSupportedChains.test.ts
│   │   │   │   ├── autoConfirmSupportedChains.ts
│   │   │   │   ├── getProvider.ts
│   │   │   │   ├── index.ts
│   │   │   │   ├── plugin.ts
│   │   │   │   └── types.ts
│   │   │   ├── ethereum
│   │   │   │   ├── strategies
│   │   │   │   │   ├── injected.test.ts
│   │   │   │   │   ├── injected.ts
│   │   │   │   │   └── types.ts
│   │   │   │   ├── chainUtils.test.ts
│   │   │   │   ├── chainUtils.ts
│   │   │   │   ├── connect.test.ts
│   │   │   │   ├── connect.ts
│   │   │   │   ├── disconnect.test.ts
│   │   │   │   ├── disconnect.ts
│   │   │   │   ├── eventListeners.test.ts
│   │   │   │   ├── eventListeners.ts
│   │   │   │   ├── getAccounts.test.ts
│   │   │   │   ├── getAccounts.ts
│   │   │   │   ├── getProvider.ts
│   │   │   │   ├── index.ts
│   │   │   │   ├── plugin.test.ts
│   │   │   │   ├── plugin.ts
│   │   │   │   ├── sendTransaction.test.ts
│   │   │   │   ├── sendTransaction.ts
│   │   │   │   ├── signIn.ts
│   │   │   │   ├── signMessage.test.ts
│   │   │   │   ├── signMessage.ts
│   │   │   │   ├── siwe.test.ts
│   │   │   │   ├── siwe.ts
│   │   │   │   └── types.ts
│   │   │   ├── extension
│   │   │   │   ├── index.ts
│   │   │   │   ├── isInstalled.ts
│   │   │   │   └── plugin.ts
│   │   │   ├── solana
│   │   │   │   ├── strategies
│   │   │   │   │   ├── injected.test.ts
│   │   │   │   │   ├── injected.ts
│   │   │   │   │   └── types.ts
│   │   │   │   ├── connect.test.ts
│   │   │   │   ├── connect.ts
│   │   │   │   ├── disconnect.test.ts
│   │   │   │   ├── disconnect.ts
│   │   │   │   ├── eventListeners.test.ts
│   │   │   │   ├── eventListeners.ts
│   │   │   │   ├── getAccount.test.ts
│   │   │   │   ├── getAccount.ts
│   │   │   │   ├── getProvider.test.ts
│   │   │   │   ├── getProvider.ts
│   │   │   │   ├── index.ts
│   │   │   │   ├── plugin.test.ts
│   │   │   │   ├── plugin.ts
│   │   │   │   ├── signAllTransactions.test.ts
│   │   │   │   ├── signAllTransactions.ts
│   │   │   │   ├── signAndSendAllTransactions.test.ts
│   │   │   │   ├── signAndSendAllTransactions.ts
│   │   │   │   ├── signAndSendTransaction.test.ts
│   │   │   │   ├── signAndSendTransaction.ts
│   │   │   │   ├── signIn.test.ts
│   │   │   │   ├── signIn.ts
│   │   │   │   ├── signMessage.test.ts
│   │   │   │   ├── signMessage.ts
│   │   │   │   ├── signTransaction.test.ts
│   │   │   │   ├── signTransaction.ts
│   │   │   │   └── types.ts
│   │   │   ├── index.ts
│   │   │   └── types.ts
│   │   ├── .eslintrc.js
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── browser-sdk
│   │   ├── src
│   │   │   ├── providers
│   │   │   │   ├── embedded
│   │   │   │   │   ├── adapters
│   │   │   │   │   │   ├── auth.ts
│   │   │   │   │   │   ├── index.ts
│   │   │   │   │   │   ├── logger.ts
│   │   │   │   │   │   ├── phantom-app.ts
│   │   │   │   │   │   ├── storage.ts
│   │   │   │   │   │   └── url-params.ts
│   │   │   │   │   ├── index.ts
│   │   │   │   │   └── storage.test.ts
│   │   │   │   └── injected
│   │   │   │       ├── chains
│   │   │   │       │   ├── ChainCallbacks.ts
│   │   │   │       │   ├── InjectedWalletEthereumChain.test.ts
│   │   │   │       │   ├── InjectedWalletEthereumChain.ts
│   │   │   │       │   ├── InjectedWalletSolanaChain.test.ts
│   │   │   │       │   ├── InjectedWalletSolanaChain.ts
│   │   │   │       │   ├── WalletStandardSolanaAdapter.test.ts
│   │   │   │       │   ├── WalletStandardSolanaAdapter.ts
│   │   │   │       │   └── walletStandardTypes.ts
│   │   │   │       ├── index.test.ts
│   │   │   │       └── index.ts
│   │   │   ├── test-utils
│   │   │   │   ├── mockWindow.ts
│   │   │   │   └── setup.ts
│   │   │   ├── utils
│   │   │   │   ├── auth-callback.test.ts
│   │   │   │   ├── auth-callback.ts
│   │   │   │   ├── browser-detection.test.ts
│   │   │   │   ├── browser-detection.ts
│   │   │   │   ├── deeplink.test.ts
│   │   │   │   └── deeplink.ts
│   │   │   ├── wallets
│   │   │   │   ├── custom-wallets.ts
│   │   │   │   ├── discovery.test.ts
│   │   │   │   ├── discovery.ts
│   │   │   │   ├── registry.test.ts
│   │   │   │   └── registry.ts
│   │   │   ├── BrowserSDK.test.ts
│   │   │   ├── BrowserSDK.ts
│   │   │   ├── ProviderManager.test.ts
│   │   │   ├── ProviderManager.ts
│   │   │   ├── debug.ts
│   │   │   ├── global.d.ts
│   │   │   ├── index.ts
│   │   │   ├── isPhantomLoginAvailable.test.ts
│   │   │   ├── isPhantomLoginAvailable.ts
│   │   │   ├── polyfills.ts
│   │   │   ├── sdk-version.d.ts
│   │   │   ├── types.ts
│   │   │   ├── waitForPhantomExtension.test.ts
│   │   │   └── waitForPhantomExtension.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── chain-interfaces
│   │   ├── src
│   │   │   ├── interfaces
│   │   │   │   ├── IEthereumChain.ts
│   │   │   │   ├── ISolanaChain.ts
│   │   │   │   └── index.ts
│   │   │   └── index.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── client
│   │   ├── src
│   │   │   ├── PhantomClient.test.ts
│   │   │   ├── PhantomClient.ts
│   │   │   ├── caip2-mappings.ts
│   │   │   ├── constants.test.ts
│   │   │   ├── constants.ts
│   │   │   ├── errors.ts
│   │   │   ├── index.ts
│   │   │   ├── types.ts
│   │   │   └── utils.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── constants
│   │   ├── src
│   │   │   ├── analytics.ts
│   │   │   ├── authenticators.ts
│   │   │   ├── environments.ts
│   │   │   ├── icons.ts
│   │   │   ├── index.ts
│   │   │   ├── network-ids.ts
│   │   │   ├── networks.test.ts
│   │   │   ├── networks.ts
│   │   │   └── provider-names.ts
│   │   ├── .eslintrc.js
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── crypto
│   │   ├── src
│   │   │   ├── crypto.test.ts
│   │   │   └── index.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── embedded-provider-core
│   │   ├── src
│   │   │   ├── chains
│   │   │   │   ├── EthereumChain.test.ts
│   │   │   │   ├── EthereumChain.ts
│   │   │   │   ├── SolanaChain.test.ts
│   │   │   │   ├── SolanaChain.ts
│   │   │   │   └── index.ts
│   │   │   ├── interfaces
│   │   │   │   ├── auth.ts
│   │   │   │   ├── index.ts
│   │   │   │   ├── platform.ts
│   │   │   │   ├── storage.ts
│   │   │   │   └── url-params.ts
│   │   │   ├── utils
│   │   │   │   ├── retry.ts
│   │   │   │   └── session.ts
│   │   │   ├── auth-flow.test.ts
│   │   │   ├── constants.ts
│   │   │   ├── embedded-provider.test.ts
│   │   │   ├── embedded-provider.ts
│   │   │   ├── index.ts
│   │   │   ├── renewal.test.ts
│   │   │   └── types.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── indexed-db-stamper
│   │   ├── src
│   │   │   ├── index.test.ts
│   │   │   ├── index.ts
│   │   │   ├── integration.test.ts
│   │   │   └── test-setup.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── mcp-server
│   │   ├── src
│   │   │   ├── auth
│   │   │   │   ├── callback-server.test.ts
│   │   │   │   ├── callback-server.ts
│   │   │   │   ├── dcr.test.ts
│   │   │   │   ├── dcr.ts
│   │   │   │   ├── oauth.test.ts
│   │   │   │   └── oauth.ts
│   │   │   ├── session
│   │   │   │   ├── manager.test.ts
│   │   │   │   ├── manager.ts
│   │   │   │   ├── storage.test.ts
│   │   │   │   ├── storage.ts
│   │   │   │   └── types.ts
│   │   │   ├── tools
│   │   │   │   ├── buy-token.ts
│   │   │   │   ├── get-wallet-addresses.ts
│   │   │   │   ├── index.ts
│   │   │   │   ├── sign-message.ts
│   │   │   │   ├── sign-transaction.ts
│   │   │   │   ├── transfer-tokens.ts
│   │   │   │   └── types.ts
│   │   │   ├── utils
│   │   │   │   ├── amount.ts
│   │   │   │   ├── logger.test.ts
│   │   │   │   ├── logger.ts
│   │   │   │   ├── network.ts
│   │   │   │   └── solana.ts
│   │   │   ├── index.ts
│   │   │   └── server.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── test-auth.js
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── parsers
│   │   ├── src
│   │   │   ├── index.ts
│   │   │   ├── parsers.integration.test.ts
│   │   │   ├── parsers.test.ts
│   │   │   ├── response-parsers.test.ts
│   │   │   └── response-parsers.ts
│   │   ├── .eslintrc.js
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── phantom-openclaw-plugin
│   │   ├── src
│   │   │   ├── client
│   │   │   │   └── types.ts
│   │   │   ├── tools
│   │   │   │   └── register-tools.ts
│   │   │   ├── index.ts
│   │   │   └── session.ts
│   │   ├── openclaw.plugin.json
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── react-native-sdk
│   │   ├── src
│   │   │   ├── components
│   │   │   │   ├── ConnectModalContent.test.tsx
│   │   │   │   ├── ConnectModalContent.tsx
│   │   │   │   ├── ConnectedModalContent.test.tsx
│   │   │   │   ├── ConnectedModalContent.tsx
│   │   │   │   ├── Modal.test.tsx
│   │   │   │   ├── Modal.tsx
│   │   │   │   ├── SpendingLimitModalContent.test.tsx
│   │   │   │   └── SpendingLimitModalContent.tsx
│   │   │   ├── hooks
│   │   │   │   ├── index.ts
│   │   │   │   ├── useAccounts.ts
│   │   │   │   ├── useConnect.ts
│   │   │   │   ├── useDisconnect.ts
│   │   │   │   ├── useEthereum.ts
│   │   │   │   └── useSolana.ts
│   │   │   ├── providers
│   │   │   │   └── embedded
│   │   │   │       ├── auth.ts
│   │   │   │       ├── logger.ts
│   │   │   │       ├── phantom-app.ts
│   │   │   │       ├── stamper.ts
│   │   │   │       ├── storage.ts
│   │   │   │       └── url-params.ts
│   │   │   ├── test
│   │   │   │   ├── mocks
│   │   │   │   │   ├── @phantom
│   │   │   │   │   │   ├── base64url.js
│   │   │   │   │   │   └── crypto.js
│   │   │   │   │   ├── expo-auth-session.js
│   │   │   │   │   ├── expo-router.js
│   │   │   │   │   ├── expo-secure-store.js
│   │   │   │   │   ├── expo-web-browser.js
│   │   │   │   │   └── react-native.js
│   │   │   │   └── setup.ts
│   │   │   ├── types
│   │   │   │   └── expo.d.ts
│   │   │   ├── ModalContext.test.tsx
│   │   │   ├── ModalContext.ts
│   │   │   ├── ModalProvider.test.tsx
│   │   │   ├── ModalProvider.tsx
│   │   │   ├── PhantomContext.tsx
│   │   │   ├── PhantomProvider.tsx
│   │   │   ├── global.d.ts
│   │   │   ├── index.test.ts
│   │   │   ├── index.ts
│   │   │   └── types.ts
│   │   ├── .eslintrc.js
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── react-sdk
│   │   ├── src
│   │   │   ├── __mocks__
│   │   │   │   └── @solana
│   │   │   │       └── web3.js.js
│   │   │   ├── components
│   │   │   │   ├── ChainIcon.tsx
│   │   │   │   ├── ConnectBox.tsx
│   │   │   │   ├── ConnectButton.tsx
│   │   │   │   ├── ConnectModalContent.test.tsx
│   │   │   │   ├── ConnectModalContent.tsx
│   │   │   │   ├── ConnectedModalContent.test.tsx
│   │   │   │   ├── ConnectedModalContent.tsx
│   │   │   │   ├── Modal.test.tsx
│   │   │   │   ├── SpendingLimitModalContent.test.tsx
│   │   │   │   ├── SpendingLimitModalContent.tsx
│   │   │   │   └── index.ts
│   │   │   ├── hooks
│   │   │   │   ├── index.ts
│   │   │   │   ├── useAccounts.test.tsx
│   │   │   │   ├── useAccounts.ts
│   │   │   │   ├── useAutoConfirm.ts
│   │   │   │   ├── useConnect.test.tsx
│   │   │   │   ├── useConnect.ts
│   │   │   │   ├── useDisconnect.test.tsx
│   │   │   │   ├── useDisconnect.ts
│   │   │   │   ├── useDiscoveredWallets.test.tsx
│   │   │   │   ├── useDiscoveredWallets.ts
│   │   │   │   ├── useEthereum.ts
│   │   │   │   ├── useIsExtensionInstalled.test.tsx
│   │   │   │   ├── useIsExtensionInstalled.ts
│   │   │   │   ├── useIsPhantomLoginAvailable.ts
│   │   │   │   └── useSolana.ts
│   │   │   ├── ModalContext.test.tsx
│   │   │   ├── ModalContext.ts
│   │   │   ├── ModalProvider.test.tsx
│   │   │   ├── ModalProvider.tsx
│   │   │   ├── PhantomContext.ts
│   │   │   ├── PhantomProvider.test.tsx
│   │   │   ├── PhantomProvider.tsx
│   │   │   ├── index.ts
│   │   │   ├── jest.d.ts
│   │   │   ├── test-setup.js
│   │   │   └── types.ts
│   │   ├── .eslintrc.js
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── tsconfig.test.json
│   │   └── tsup.config.ts
│   ├── sdk-types
│   │   ├── src
│   │   │   ├── index.test.ts
│   │   │   └── index.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── server-sdk
│   │   ├── src
│   │   │   ├── index.ts
│   │   │   └── types.ts
│   │   ├── tests
│   │   │   ├── server-sdk.test.ts
│   │   │   └── setup.ts
│   │   ├── jest.config.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   ├── ui
│   │   ├── src
│   │   │   ├── components
│   │   │   │   ├── BoundedIcon.native.tsx
│   │   │   │   ├── BoundedIcon.ts
│   │   │   │   ├── BoundedIcon.web.tsx
│   │   │   │   ├── Button.native.tsx
│   │   │   │   ├── Button.ts
│   │   │   │   ├── Button.web.tsx
│   │   │   │   ├── Icon.native.tsx
│   │   │   │   ├── Icon.ts
│   │   │   │   ├── Icon.web.tsx
│   │   │   │   ├── Modal.native.tsx
│   │   │   │   ├── Modal.ts
│   │   │   │   ├── Modal.web.tsx
│   │   │   │   ├── ModalHeader.native.tsx
│   │   │   │   ├── ModalHeader.ts
│   │   │   │   ├── ModalHeader.web.tsx
│   │   │   │   ├── Skeleton.native.tsx
│   │   │   │   ├── Skeleton.ts
│   │   │   │   ├── Skeleton.web.tsx
│   │   │   │   ├── Text.native.tsx
│   │   │   │   ├── Text.ts
│   │   │   │   └── Text.web.tsx
│   │   │   ├── hooks
│   │   │   │   └── useTheme.ts
│   │   │   ├── themes
│   │   │   │   ├── ThemeContext.native.tsx
│   │   │   │   ├── ThemeContext.tsx
│   │   │   │   └── index.ts
│   │   │   ├── utils
│   │   │   │   └── index.ts
│   │   │   └── index.ts
│   │   ├── .eslintrc.js
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── tsup.config.ts
│   └── utils
│       ├── src
│       │   ├── index.ts
│       │   ├── network.test.ts
│       │   ├── network.ts
│       │   ├── time.test.ts
│       │   ├── time.ts
│       │   ├── uuid.test.ts
│       │   └── uuid.ts
│       ├── jest.config.js
│       ├── package.json
│       └── tsconfig.json
├── .eslintrc.js
├── .prettierrc.js
├── .yarnrc.yml
├── jest.config.js
├── package.json
├── sharedJestConfig.js
├── tsconfig.json
└── turbo.json
```
