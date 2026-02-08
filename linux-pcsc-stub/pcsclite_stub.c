// Stub library for cross-compiling pcsc-sys for Linux from macOS.
// At runtime, the real libpcsclite.so from the system is used.

void SCardEstablishContext() {}
void SCardReleaseContext() {}
void SCardListReaders() {}
void SCardConnect() {}
void SCardDisconnect() {}
void SCardTransmit() {}
void SCardGetStatusChange() {}
void SCardBeginTransaction() {}
void SCardEndTransaction() {}
void SCardStatus() {}
void SCardReconnect() {}
void SCardCancel() {}
void SCardIsValidContext() {}
void SCardFreeMemory() {}
void SCardListReaderGroups() {}
void SCardGetAttrib() {}
void SCardSetAttrib() {}
void SCardControl() {}

// Global PCI structs referenced by pcsc crate
typedef struct { unsigned long dwProtocol; unsigned long cbPciLength; } SCARD_IO_REQUEST;
SCARD_IO_REQUEST g_rgSCardT0Pci = {1, 8};
SCARD_IO_REQUEST g_rgSCardT1Pci = {2, 8};
SCARD_IO_REQUEST g_rgSCardRawPci = {0, 8};
