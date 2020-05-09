use libdma::nvme::NvmeCommand;

#[repr(u8)]
enum AdminCommandSet {
    DELETE_IO_SQ = 0x00,
    CREATE_IO_SQ = 0x01,
    DELETE_IO_CQ = 0x04,
    CREATE_IO_CQ = 0x05,
    IDENTIFY_COMMAND = 0x06,
}

#[repr(u8)]
enum IoCommandSet {
    FLUSH = 0x00,
    WRITE = 0x01,
    READ = 0x02,
}

/// implementation of NvmeCommand
pub fn create_io_completion_queue(cid: u16, qid: u16, ptr: usize, size: u16) -> NvmeCommand {
    NvmeCommand {
        opcode: AdminCommandSet::CREATE_IO_CQ as u8,
        flags: 0,
        cid: cid,
        nsid: 0,
        _rsvd: 0,
        mptr: 0,
        dptr: [ptr as u64, 0],
        cdw10: ((size as u32) << 16) | (qid as u32),
        cdw11: 1 /* Physically Contiguous */, //TODO: IV, IEN
        cdw12: 0,
        cdw13: 0,
        cdw14: 0,
        cdw15: 0,
    }
}

pub fn create_io_submission_queue(cid: u16, qid: u16, ptr: usize, size: u16, cqid: u16) -> NvmeCommand {
    NvmeCommand {
        opcode: AdminCommandSet::CREATE_IO_SQ as u8,
        flags: 0,
        cid: cid,
        nsid: 0,
        _rsvd: 0,
        mptr: 0,
        dptr: [ptr as u64, 0],
        cdw10: ((size as u32) << 16) | (qid as u32),
        cdw11: ((cqid as u32) << 16) | 1 /* Physically Contiguous */, //TODO: QPRIO
        cdw12: 0, //TODO: NVMSETID
        cdw13: 0,
        cdw14: 0,
        cdw15: 0,
    }
}

pub fn identify_namespace(cid: u16, ptr: usize, nsid: u32) -> NvmeCommand {
    NvmeCommand {
        opcode: AdminCommandSet::IDENTIFY_COMMAND as u8,
        flags: 0,
        cid: cid,
        nsid: nsid,
        _rsvd: 0,
        mptr: 0,
        dptr: [ptr as u64, 0],
        cdw10: 0,
        cdw11: 0,
        cdw12: 0,
        cdw13: 0,
        cdw14: 0,
        cdw15: 0,
    }
}

pub fn identify_controller(cid: u16, ptr: usize) -> NvmeCommand {
    NvmeCommand {
        opcode: AdminCommandSet::IDENTIFY_COMMAND as u8,
        flags: 0,
        cid: cid,
        nsid: 0,
        _rsvd: 0,
        mptr: 0,
        dptr: [ptr as u64, 0],
        cdw10: 1,
        cdw11: 0,
        cdw12: 0,
        cdw13: 0,
        cdw14: 0,
        cdw15: 0,
    }
}

pub fn identify_namespace_list(cid: u16, ptr: usize, base: u32) -> NvmeCommand {
    NvmeCommand {
        opcode: AdminCommandSet::IDENTIFY_COMMAND as u8,
        flags: 0,
        cid: cid,
        nsid: base,
        _rsvd: 0,
        mptr: 0,
        dptr: [ptr as u64, 0],
        cdw10: 2,
        cdw11: 0,
        cdw12: 0,
        cdw13: 0,
        cdw14: 0,
        cdw15: 0,
    }
}

pub fn io_read(cid: u16, nsid: u32, lba: u64, blocks_1: u16, ptr0: u64, ptr1: u64) -> NvmeCommand {
    NvmeCommand {
        opcode: IoCommandSet::READ as u8,
        flags: 1 << 6,
        cid: cid,
        nsid: nsid,
        _rsvd: 0,
        mptr: 0,
        dptr: [ptr0, ptr1],
        cdw10: lba as u32,
        cdw11: (lba >> 32) as u32,
        cdw12: blocks_1 as u32,
        cdw13: 0,
        cdw14: 0,
        cdw15: 0,
    }
}

pub fn io_write(cid: u16, nsid: u32, lba: u64, blocks_1: u16, ptr0: u64, ptr1: u64) -> NvmeCommand {
    NvmeCommand {
        opcode: IoCommandSet::WRITE as u8,
        flags:  1 << 6,
        cid: cid,
        nsid: nsid,
        _rsvd: 0,
        mptr: 0,
        dptr: [ptr0, ptr1],
        cdw10: lba as u32,
        cdw11: (lba >> 32) as u32,
        cdw12: blocks_1 as u32,
        cdw13: 0,
        cdw14: 0,
        cdw15: 0,
    }
}
