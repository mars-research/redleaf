use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

// Use the PIC 8259 crate 
// https://docs.rs/crate/pic8259_simple/0.1.1/source/src/lib.rs
use pic8259_simple::ChainedPics;
use spin;

use crate::{gdt, lapic, println};

// Map first PIC line to interrupt 32 
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

macro_rules! dummy_interrupt_handler {
    ($name: ident, $interrupt: expr) => {
        extern "x86-interrupt" fn $name(stack_frame: &mut InterruptStackFrame) {
            println!("Interrupt {} triggered", $interrupt);
            lapic::end_of_interrupt();
            //unsafe {
            //   PICS.lock()
            //        .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
            //}

        }
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// See https://os.phil-opp.com/hardware-interrupts/ 
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });


lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        idt.non_maskable_interrupt.set_handler_fn(nmi_handler);

		/* NMI fault hanler executes on the IST stack */
        //unsafe {
        //    idt.non_maskable_interrupt
		//		.set_handler_fn(nmi_handler)
        //        .set_stack_index(gdt::NMI_IST_INDEX); 
        //}

		/* Double fault hanler executes on the IST stack -- just in 
 		   case the kernel stack is already full and triggers a pagefault, 
		   that in turn (since the hardware will not be able to push the 
  	 	   exception fault on the stack will trigger a tripple fault */
        unsafe {
            idt.double_fault
				.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); 
        }

        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[19].set_handler_fn(apic_error_handler);

idt[32].set_handler_fn(dummy_int_32_handler);
idt[33].set_handler_fn(dummy_int_33_handler);
idt[34].set_handler_fn(dummy_int_34_handler);
idt[35].set_handler_fn(dummy_int_35_handler);
idt[36].set_handler_fn(dummy_int_36_handler);
idt[37].set_handler_fn(dummy_int_37_handler);
idt[38].set_handler_fn(dummy_int_38_handler);
idt[39].set_handler_fn(dummy_int_39_handler);
idt[40].set_handler_fn(dummy_int_40_handler);
idt[41].set_handler_fn(dummy_int_41_handler);
idt[42].set_handler_fn(dummy_int_42_handler);
idt[43].set_handler_fn(dummy_int_43_handler);
idt[44].set_handler_fn(dummy_int_44_handler);
idt[45].set_handler_fn(dummy_int_45_handler);
idt[46].set_handler_fn(dummy_int_46_handler);
idt[47].set_handler_fn(dummy_int_47_handler);
idt[48].set_handler_fn(dummy_int_48_handler);
idt[49].set_handler_fn(dummy_int_49_handler);
idt[50].set_handler_fn(dummy_int_50_handler);
idt[51].set_handler_fn(dummy_int_51_handler);
idt[52].set_handler_fn(dummy_int_52_handler);
idt[53].set_handler_fn(dummy_int_53_handler);
idt[54].set_handler_fn(dummy_int_54_handler);
idt[55].set_handler_fn(dummy_int_55_handler);
idt[56].set_handler_fn(dummy_int_56_handler);
idt[57].set_handler_fn(dummy_int_57_handler);
idt[58].set_handler_fn(dummy_int_58_handler);
idt[59].set_handler_fn(dummy_int_59_handler);
idt[60].set_handler_fn(dummy_int_60_handler);
idt[61].set_handler_fn(dummy_int_61_handler);
idt[62].set_handler_fn(dummy_int_62_handler);
idt[63].set_handler_fn(dummy_int_63_handler);
idt[64].set_handler_fn(dummy_int_64_handler);
idt[65].set_handler_fn(dummy_int_65_handler);
idt[66].set_handler_fn(dummy_int_66_handler);
idt[67].set_handler_fn(dummy_int_67_handler);
idt[68].set_handler_fn(dummy_int_68_handler);
idt[69].set_handler_fn(dummy_int_69_handler);
idt[70].set_handler_fn(dummy_int_70_handler);
idt[71].set_handler_fn(dummy_int_71_handler);
idt[72].set_handler_fn(dummy_int_72_handler);
idt[73].set_handler_fn(dummy_int_73_handler);
idt[74].set_handler_fn(dummy_int_74_handler);
idt[75].set_handler_fn(dummy_int_75_handler);
idt[76].set_handler_fn(dummy_int_76_handler);
idt[77].set_handler_fn(dummy_int_77_handler);
idt[78].set_handler_fn(dummy_int_78_handler);
idt[79].set_handler_fn(dummy_int_79_handler);
idt[80].set_handler_fn(dummy_int_80_handler);
idt[81].set_handler_fn(dummy_int_81_handler);
idt[82].set_handler_fn(dummy_int_82_handler);
idt[83].set_handler_fn(dummy_int_83_handler);
idt[84].set_handler_fn(dummy_int_84_handler);
idt[85].set_handler_fn(dummy_int_85_handler);
idt[86].set_handler_fn(dummy_int_86_handler);
idt[87].set_handler_fn(dummy_int_87_handler);
idt[88].set_handler_fn(dummy_int_88_handler);
idt[89].set_handler_fn(dummy_int_89_handler);
idt[90].set_handler_fn(dummy_int_90_handler);
idt[91].set_handler_fn(dummy_int_91_handler);
idt[92].set_handler_fn(dummy_int_92_handler);
idt[93].set_handler_fn(dummy_int_93_handler);
idt[94].set_handler_fn(dummy_int_94_handler);
idt[95].set_handler_fn(dummy_int_95_handler);
idt[96].set_handler_fn(dummy_int_96_handler);
idt[97].set_handler_fn(dummy_int_97_handler);
idt[98].set_handler_fn(dummy_int_98_handler);
idt[99].set_handler_fn(dummy_int_99_handler);
idt[100].set_handler_fn(dummy_int_100_handler);
idt[101].set_handler_fn(dummy_int_101_handler);
idt[102].set_handler_fn(dummy_int_102_handler);
idt[103].set_handler_fn(dummy_int_103_handler);
idt[104].set_handler_fn(dummy_int_104_handler);
idt[105].set_handler_fn(dummy_int_105_handler);
idt[106].set_handler_fn(dummy_int_106_handler);
idt[107].set_handler_fn(dummy_int_107_handler);
idt[108].set_handler_fn(dummy_int_108_handler);
idt[109].set_handler_fn(dummy_int_109_handler);
idt[110].set_handler_fn(dummy_int_110_handler);
idt[111].set_handler_fn(dummy_int_111_handler);
idt[112].set_handler_fn(dummy_int_112_handler);
idt[113].set_handler_fn(dummy_int_113_handler);
idt[114].set_handler_fn(dummy_int_114_handler);
idt[115].set_handler_fn(dummy_int_115_handler);
idt[116].set_handler_fn(dummy_int_116_handler);
idt[117].set_handler_fn(dummy_int_117_handler);
idt[118].set_handler_fn(dummy_int_118_handler);
idt[119].set_handler_fn(dummy_int_119_handler);
idt[120].set_handler_fn(dummy_int_120_handler);
idt[121].set_handler_fn(dummy_int_121_handler);
idt[122].set_handler_fn(dummy_int_122_handler);
idt[123].set_handler_fn(dummy_int_123_handler);
idt[124].set_handler_fn(dummy_int_124_handler);
idt[125].set_handler_fn(dummy_int_125_handler);
idt[126].set_handler_fn(dummy_int_126_handler);
idt[127].set_handler_fn(dummy_int_127_handler);
idt[128].set_handler_fn(dummy_int_128_handler);
idt[129].set_handler_fn(dummy_int_129_handler);
idt[130].set_handler_fn(dummy_int_130_handler);
idt[131].set_handler_fn(dummy_int_131_handler);
idt[132].set_handler_fn(dummy_int_132_handler);
idt[133].set_handler_fn(dummy_int_133_handler);
idt[134].set_handler_fn(dummy_int_134_handler);
idt[135].set_handler_fn(dummy_int_135_handler);
idt[136].set_handler_fn(dummy_int_136_handler);
idt[137].set_handler_fn(dummy_int_137_handler);
idt[138].set_handler_fn(dummy_int_138_handler);
idt[139].set_handler_fn(dummy_int_139_handler);
idt[140].set_handler_fn(dummy_int_140_handler);
idt[141].set_handler_fn(dummy_int_141_handler);
idt[142].set_handler_fn(dummy_int_142_handler);
idt[143].set_handler_fn(dummy_int_143_handler);
idt[144].set_handler_fn(dummy_int_144_handler);
idt[145].set_handler_fn(dummy_int_145_handler);
idt[146].set_handler_fn(dummy_int_146_handler);
idt[147].set_handler_fn(dummy_int_147_handler);
idt[148].set_handler_fn(dummy_int_148_handler);
idt[149].set_handler_fn(dummy_int_149_handler);
idt[150].set_handler_fn(dummy_int_150_handler);
idt[151].set_handler_fn(dummy_int_151_handler);
idt[152].set_handler_fn(dummy_int_152_handler);
idt[153].set_handler_fn(dummy_int_153_handler);
idt[154].set_handler_fn(dummy_int_154_handler);
idt[155].set_handler_fn(dummy_int_155_handler);
idt[156].set_handler_fn(dummy_int_156_handler);
idt[157].set_handler_fn(dummy_int_157_handler);
idt[158].set_handler_fn(dummy_int_158_handler);
idt[159].set_handler_fn(dummy_int_159_handler);
idt[160].set_handler_fn(dummy_int_160_handler);
idt[161].set_handler_fn(dummy_int_161_handler);
idt[162].set_handler_fn(dummy_int_162_handler);
idt[163].set_handler_fn(dummy_int_163_handler);
idt[164].set_handler_fn(dummy_int_164_handler);
idt[165].set_handler_fn(dummy_int_165_handler);
idt[166].set_handler_fn(dummy_int_166_handler);
idt[167].set_handler_fn(dummy_int_167_handler);
idt[168].set_handler_fn(dummy_int_168_handler);
idt[169].set_handler_fn(dummy_int_169_handler);
idt[170].set_handler_fn(dummy_int_170_handler);
idt[171].set_handler_fn(dummy_int_171_handler);
idt[172].set_handler_fn(dummy_int_172_handler);
idt[173].set_handler_fn(dummy_int_173_handler);
idt[174].set_handler_fn(dummy_int_174_handler);
idt[175].set_handler_fn(dummy_int_175_handler);
idt[176].set_handler_fn(dummy_int_176_handler);
idt[177].set_handler_fn(dummy_int_177_handler);
idt[178].set_handler_fn(dummy_int_178_handler);
idt[179].set_handler_fn(dummy_int_179_handler);
idt[180].set_handler_fn(dummy_int_180_handler);
idt[181].set_handler_fn(dummy_int_181_handler);
idt[182].set_handler_fn(dummy_int_182_handler);
idt[183].set_handler_fn(dummy_int_183_handler);
idt[184].set_handler_fn(dummy_int_184_handler);
idt[185].set_handler_fn(dummy_int_185_handler);
idt[186].set_handler_fn(dummy_int_186_handler);
idt[187].set_handler_fn(dummy_int_187_handler);
idt[188].set_handler_fn(dummy_int_188_handler);
idt[189].set_handler_fn(dummy_int_189_handler);
idt[190].set_handler_fn(dummy_int_190_handler);
idt[191].set_handler_fn(dummy_int_191_handler);
idt[192].set_handler_fn(dummy_int_192_handler);
idt[193].set_handler_fn(dummy_int_193_handler);
idt[194].set_handler_fn(dummy_int_194_handler);
idt[195].set_handler_fn(dummy_int_195_handler);
idt[196].set_handler_fn(dummy_int_196_handler);
idt[197].set_handler_fn(dummy_int_197_handler);
idt[198].set_handler_fn(dummy_int_198_handler);
idt[199].set_handler_fn(dummy_int_199_handler);
idt[200].set_handler_fn(dummy_int_200_handler);
idt[201].set_handler_fn(dummy_int_201_handler);
idt[202].set_handler_fn(dummy_int_202_handler);
idt[203].set_handler_fn(dummy_int_203_handler);
idt[204].set_handler_fn(dummy_int_204_handler);
idt[205].set_handler_fn(dummy_int_205_handler);
idt[206].set_handler_fn(dummy_int_206_handler);
idt[207].set_handler_fn(dummy_int_207_handler);
idt[208].set_handler_fn(dummy_int_208_handler);
idt[209].set_handler_fn(dummy_int_209_handler);
idt[210].set_handler_fn(dummy_int_210_handler);
idt[211].set_handler_fn(dummy_int_211_handler);
idt[212].set_handler_fn(dummy_int_212_handler);
idt[213].set_handler_fn(dummy_int_213_handler);
idt[214].set_handler_fn(dummy_int_214_handler);
idt[215].set_handler_fn(dummy_int_215_handler);
idt[216].set_handler_fn(dummy_int_216_handler);
idt[217].set_handler_fn(dummy_int_217_handler);
idt[218].set_handler_fn(dummy_int_218_handler);
idt[219].set_handler_fn(dummy_int_219_handler);
idt[220].set_handler_fn(dummy_int_220_handler);
idt[221].set_handler_fn(dummy_int_221_handler);
idt[222].set_handler_fn(dummy_int_222_handler);
idt[223].set_handler_fn(dummy_int_223_handler);
idt[224].set_handler_fn(dummy_int_224_handler);
idt[225].set_handler_fn(dummy_int_225_handler);
idt[226].set_handler_fn(dummy_int_226_handler);
idt[227].set_handler_fn(dummy_int_227_handler);
idt[228].set_handler_fn(dummy_int_228_handler);
idt[229].set_handler_fn(dummy_int_229_handler);
idt[230].set_handler_fn(dummy_int_230_handler);
idt[231].set_handler_fn(dummy_int_231_handler);
idt[232].set_handler_fn(dummy_int_232_handler);
idt[233].set_handler_fn(dummy_int_233_handler);
idt[234].set_handler_fn(dummy_int_234_handler);
idt[235].set_handler_fn(dummy_int_235_handler);
idt[236].set_handler_fn(dummy_int_236_handler);
idt[237].set_handler_fn(dummy_int_237_handler);
idt[238].set_handler_fn(dummy_int_238_handler);
idt[239].set_handler_fn(dummy_int_239_handler);
idt[240].set_handler_fn(dummy_int_240_handler);
idt[241].set_handler_fn(dummy_int_241_handler);
idt[242].set_handler_fn(dummy_int_242_handler);
idt[243].set_handler_fn(dummy_int_243_handler);
idt[244].set_handler_fn(dummy_int_244_handler);
idt[245].set_handler_fn(dummy_int_245_handler);
idt[246].set_handler_fn(dummy_int_246_handler);
idt[247].set_handler_fn(dummy_int_247_handler);
idt[248].set_handler_fn(dummy_int_248_handler);
idt[249].set_handler_fn(dummy_int_249_handler);
idt[250].set_handler_fn(dummy_int_250_handler);
idt[251].set_handler_fn(dummy_int_251_handler);
idt[252].set_handler_fn(dummy_int_252_handler);
idt[253].set_handler_fn(dummy_int_253_handler);
idt[254].set_handler_fn(dummy_int_254_handler);
idt[255].set_handler_fn(dummy_int_255_handler);

        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

pub fn init_irqs() {
    lapic::init();
      //unsafe { PICS.lock().initialize() };
}


extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("breakpoint:\n{:#?}", stack_frame);
    lapic::end_of_interrupt();
}

extern "x86-interrupt" fn nmi_handler(
    stack_frame: &mut InterruptStackFrame,
) {
    println!("nmi:\n{:#?}", stack_frame); 
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("double fault:\n{:#?}", stack_frame);
	crate::halt(); 
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("segment not present:\n{:#?}", stack_frame);
	crate::halt(); 
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("stack segment fault:\n{:#?}", stack_frame);
	crate::halt(); 
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("general protection fault:\n{:#?}", stack_frame);
	crate::halt(); 
}

extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("alignment check:\n{:#?}", stack_frame);
	crate::halt(); 
}

extern "x86-interrupt" fn machine_check_handler(
    stack_frame: &mut InterruptStackFrame,
) {
    println!("machine check:\n{:#?}", stack_frame);
	crate::halt(); 
}

use x86_64::structures::idt::PageFaultErrorCode;

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    crate::halt();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    print!(".");

    /*
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }*/
}

extern "x86-interrupt" fn apic_error_handler(stack_frame: &mut InterruptStackFrame) {
    println!("apic error:\n{:#?}", stack_frame);
    lapic::end_of_interrupt();

    /*
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }*/
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    /*
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
    */
}

dummy_interrupt_handler!(dummy_int_32_handler, 32);
dummy_interrupt_handler!(dummy_int_33_handler, 33);
dummy_interrupt_handler!(dummy_int_34_handler, 34);
dummy_interrupt_handler!(dummy_int_35_handler, 35);
dummy_interrupt_handler!(dummy_int_36_handler, 36);
dummy_interrupt_handler!(dummy_int_37_handler, 37);
dummy_interrupt_handler!(dummy_int_38_handler, 38);
dummy_interrupt_handler!(dummy_int_39_handler, 39);
dummy_interrupt_handler!(dummy_int_40_handler, 40);
dummy_interrupt_handler!(dummy_int_41_handler, 41);
dummy_interrupt_handler!(dummy_int_42_handler, 42);
dummy_interrupt_handler!(dummy_int_43_handler, 43);
dummy_interrupt_handler!(dummy_int_44_handler, 44);
dummy_interrupt_handler!(dummy_int_45_handler, 45);
dummy_interrupt_handler!(dummy_int_46_handler, 46);
dummy_interrupt_handler!(dummy_int_47_handler, 47);
dummy_interrupt_handler!(dummy_int_48_handler, 48);
dummy_interrupt_handler!(dummy_int_49_handler, 49);
dummy_interrupt_handler!(dummy_int_50_handler, 50);
dummy_interrupt_handler!(dummy_int_51_handler, 51);
dummy_interrupt_handler!(dummy_int_52_handler, 52);
dummy_interrupt_handler!(dummy_int_53_handler, 53);
dummy_interrupt_handler!(dummy_int_54_handler, 54);
dummy_interrupt_handler!(dummy_int_55_handler, 55);
dummy_interrupt_handler!(dummy_int_56_handler, 56);
dummy_interrupt_handler!(dummy_int_57_handler, 57);
dummy_interrupt_handler!(dummy_int_58_handler, 58);
dummy_interrupt_handler!(dummy_int_59_handler, 59);
dummy_interrupt_handler!(dummy_int_60_handler, 60);
dummy_interrupt_handler!(dummy_int_61_handler, 61);
dummy_interrupt_handler!(dummy_int_62_handler, 62);
dummy_interrupt_handler!(dummy_int_63_handler, 63);
dummy_interrupt_handler!(dummy_int_64_handler, 64);
dummy_interrupt_handler!(dummy_int_65_handler, 65);
dummy_interrupt_handler!(dummy_int_66_handler, 66);
dummy_interrupt_handler!(dummy_int_67_handler, 67);
dummy_interrupt_handler!(dummy_int_68_handler, 68);
dummy_interrupt_handler!(dummy_int_69_handler, 69);
dummy_interrupt_handler!(dummy_int_70_handler, 70);
dummy_interrupt_handler!(dummy_int_71_handler, 71);
dummy_interrupt_handler!(dummy_int_72_handler, 72);
dummy_interrupt_handler!(dummy_int_73_handler, 73);
dummy_interrupt_handler!(dummy_int_74_handler, 74);
dummy_interrupt_handler!(dummy_int_75_handler, 75);
dummy_interrupt_handler!(dummy_int_76_handler, 76);
dummy_interrupt_handler!(dummy_int_77_handler, 77);
dummy_interrupt_handler!(dummy_int_78_handler, 78);
dummy_interrupt_handler!(dummy_int_79_handler, 79);
dummy_interrupt_handler!(dummy_int_80_handler, 80);
dummy_interrupt_handler!(dummy_int_81_handler, 81);
dummy_interrupt_handler!(dummy_int_82_handler, 82);
dummy_interrupt_handler!(dummy_int_83_handler, 83);
dummy_interrupt_handler!(dummy_int_84_handler, 84);
dummy_interrupt_handler!(dummy_int_85_handler, 85);
dummy_interrupt_handler!(dummy_int_86_handler, 86);
dummy_interrupt_handler!(dummy_int_87_handler, 87);
dummy_interrupt_handler!(dummy_int_88_handler, 88);
dummy_interrupt_handler!(dummy_int_89_handler, 89);
dummy_interrupt_handler!(dummy_int_90_handler, 90);
dummy_interrupt_handler!(dummy_int_91_handler, 91);
dummy_interrupt_handler!(dummy_int_92_handler, 92);
dummy_interrupt_handler!(dummy_int_93_handler, 93);
dummy_interrupt_handler!(dummy_int_94_handler, 94);
dummy_interrupt_handler!(dummy_int_95_handler, 95);
dummy_interrupt_handler!(dummy_int_96_handler, 96);
dummy_interrupt_handler!(dummy_int_97_handler, 97);
dummy_interrupt_handler!(dummy_int_98_handler, 98);
dummy_interrupt_handler!(dummy_int_99_handler, 99);
dummy_interrupt_handler!(dummy_int_100_handler, 100);
dummy_interrupt_handler!(dummy_int_101_handler, 101);
dummy_interrupt_handler!(dummy_int_102_handler, 102);
dummy_interrupt_handler!(dummy_int_103_handler, 103);
dummy_interrupt_handler!(dummy_int_104_handler, 104);
dummy_interrupt_handler!(dummy_int_105_handler, 105);
dummy_interrupt_handler!(dummy_int_106_handler, 106);
dummy_interrupt_handler!(dummy_int_107_handler, 107);
dummy_interrupt_handler!(dummy_int_108_handler, 108);
dummy_interrupt_handler!(dummy_int_109_handler, 109);
dummy_interrupt_handler!(dummy_int_110_handler, 110);
dummy_interrupt_handler!(dummy_int_111_handler, 111);
dummy_interrupt_handler!(dummy_int_112_handler, 112);
dummy_interrupt_handler!(dummy_int_113_handler, 113);
dummy_interrupt_handler!(dummy_int_114_handler, 114);
dummy_interrupt_handler!(dummy_int_115_handler, 115);
dummy_interrupt_handler!(dummy_int_116_handler, 116);
dummy_interrupt_handler!(dummy_int_117_handler, 117);
dummy_interrupt_handler!(dummy_int_118_handler, 118);
dummy_interrupt_handler!(dummy_int_119_handler, 119);
dummy_interrupt_handler!(dummy_int_120_handler, 120);
dummy_interrupt_handler!(dummy_int_121_handler, 121);
dummy_interrupt_handler!(dummy_int_122_handler, 122);
dummy_interrupt_handler!(dummy_int_123_handler, 123);
dummy_interrupt_handler!(dummy_int_124_handler, 124);
dummy_interrupt_handler!(dummy_int_125_handler, 125);
dummy_interrupt_handler!(dummy_int_126_handler, 126);
dummy_interrupt_handler!(dummy_int_127_handler, 127);
dummy_interrupt_handler!(dummy_int_128_handler, 128);
dummy_interrupt_handler!(dummy_int_129_handler, 129);
dummy_interrupt_handler!(dummy_int_130_handler, 130);
dummy_interrupt_handler!(dummy_int_131_handler, 131);
dummy_interrupt_handler!(dummy_int_132_handler, 132);
dummy_interrupt_handler!(dummy_int_133_handler, 133);
dummy_interrupt_handler!(dummy_int_134_handler, 134);
dummy_interrupt_handler!(dummy_int_135_handler, 135);
dummy_interrupt_handler!(dummy_int_136_handler, 136);
dummy_interrupt_handler!(dummy_int_137_handler, 137);
dummy_interrupt_handler!(dummy_int_138_handler, 138);
dummy_interrupt_handler!(dummy_int_139_handler, 139);
dummy_interrupt_handler!(dummy_int_140_handler, 140);
dummy_interrupt_handler!(dummy_int_141_handler, 141);
dummy_interrupt_handler!(dummy_int_142_handler, 142);
dummy_interrupt_handler!(dummy_int_143_handler, 143);
dummy_interrupt_handler!(dummy_int_144_handler, 144);
dummy_interrupt_handler!(dummy_int_145_handler, 145);
dummy_interrupt_handler!(dummy_int_146_handler, 146);
dummy_interrupt_handler!(dummy_int_147_handler, 147);
dummy_interrupt_handler!(dummy_int_148_handler, 148);
dummy_interrupt_handler!(dummy_int_149_handler, 149);
dummy_interrupt_handler!(dummy_int_150_handler, 150);
dummy_interrupt_handler!(dummy_int_151_handler, 151);
dummy_interrupt_handler!(dummy_int_152_handler, 152);
dummy_interrupt_handler!(dummy_int_153_handler, 153);
dummy_interrupt_handler!(dummy_int_154_handler, 154);
dummy_interrupt_handler!(dummy_int_155_handler, 155);
dummy_interrupt_handler!(dummy_int_156_handler, 156);
dummy_interrupt_handler!(dummy_int_157_handler, 157);
dummy_interrupt_handler!(dummy_int_158_handler, 158);
dummy_interrupt_handler!(dummy_int_159_handler, 159);
dummy_interrupt_handler!(dummy_int_160_handler, 160);
dummy_interrupt_handler!(dummy_int_161_handler, 161);
dummy_interrupt_handler!(dummy_int_162_handler, 162);
dummy_interrupt_handler!(dummy_int_163_handler, 163);
dummy_interrupt_handler!(dummy_int_164_handler, 164);
dummy_interrupt_handler!(dummy_int_165_handler, 165);
dummy_interrupt_handler!(dummy_int_166_handler, 166);
dummy_interrupt_handler!(dummy_int_167_handler, 167);
dummy_interrupt_handler!(dummy_int_168_handler, 168);
dummy_interrupt_handler!(dummy_int_169_handler, 169);
dummy_interrupt_handler!(dummy_int_170_handler, 170);
dummy_interrupt_handler!(dummy_int_171_handler, 171);
dummy_interrupt_handler!(dummy_int_172_handler, 172);
dummy_interrupt_handler!(dummy_int_173_handler, 173);
dummy_interrupt_handler!(dummy_int_174_handler, 174);
dummy_interrupt_handler!(dummy_int_175_handler, 175);
dummy_interrupt_handler!(dummy_int_176_handler, 176);
dummy_interrupt_handler!(dummy_int_177_handler, 177);
dummy_interrupt_handler!(dummy_int_178_handler, 178);
dummy_interrupt_handler!(dummy_int_179_handler, 179);
dummy_interrupt_handler!(dummy_int_180_handler, 180);
dummy_interrupt_handler!(dummy_int_181_handler, 181);
dummy_interrupt_handler!(dummy_int_182_handler, 182);
dummy_interrupt_handler!(dummy_int_183_handler, 183);
dummy_interrupt_handler!(dummy_int_184_handler, 184);
dummy_interrupt_handler!(dummy_int_185_handler, 185);
dummy_interrupt_handler!(dummy_int_186_handler, 186);
dummy_interrupt_handler!(dummy_int_187_handler, 187);
dummy_interrupt_handler!(dummy_int_188_handler, 188);
dummy_interrupt_handler!(dummy_int_189_handler, 189);
dummy_interrupt_handler!(dummy_int_190_handler, 190);
dummy_interrupt_handler!(dummy_int_191_handler, 191);
dummy_interrupt_handler!(dummy_int_192_handler, 192);
dummy_interrupt_handler!(dummy_int_193_handler, 193);
dummy_interrupt_handler!(dummy_int_194_handler, 194);
dummy_interrupt_handler!(dummy_int_195_handler, 195);
dummy_interrupt_handler!(dummy_int_196_handler, 196);
dummy_interrupt_handler!(dummy_int_197_handler, 197);
dummy_interrupt_handler!(dummy_int_198_handler, 198);
dummy_interrupt_handler!(dummy_int_199_handler, 199);
dummy_interrupt_handler!(dummy_int_200_handler, 200);
dummy_interrupt_handler!(dummy_int_201_handler, 201);
dummy_interrupt_handler!(dummy_int_202_handler, 202);
dummy_interrupt_handler!(dummy_int_203_handler, 203);
dummy_interrupt_handler!(dummy_int_204_handler, 204);
dummy_interrupt_handler!(dummy_int_205_handler, 205);
dummy_interrupt_handler!(dummy_int_206_handler, 206);
dummy_interrupt_handler!(dummy_int_207_handler, 207);
dummy_interrupt_handler!(dummy_int_208_handler, 208);
dummy_interrupt_handler!(dummy_int_209_handler, 209);
dummy_interrupt_handler!(dummy_int_210_handler, 210);
dummy_interrupt_handler!(dummy_int_211_handler, 211);
dummy_interrupt_handler!(dummy_int_212_handler, 212);
dummy_interrupt_handler!(dummy_int_213_handler, 213);
dummy_interrupt_handler!(dummy_int_214_handler, 214);
dummy_interrupt_handler!(dummy_int_215_handler, 215);
dummy_interrupt_handler!(dummy_int_216_handler, 216);
dummy_interrupt_handler!(dummy_int_217_handler, 217);
dummy_interrupt_handler!(dummy_int_218_handler, 218);
dummy_interrupt_handler!(dummy_int_219_handler, 219);
dummy_interrupt_handler!(dummy_int_220_handler, 220);
dummy_interrupt_handler!(dummy_int_221_handler, 221);
dummy_interrupt_handler!(dummy_int_222_handler, 222);
dummy_interrupt_handler!(dummy_int_223_handler, 223);
dummy_interrupt_handler!(dummy_int_224_handler, 224);
dummy_interrupt_handler!(dummy_int_225_handler, 225);
dummy_interrupt_handler!(dummy_int_226_handler, 226);
dummy_interrupt_handler!(dummy_int_227_handler, 227);
dummy_interrupt_handler!(dummy_int_228_handler, 228);
dummy_interrupt_handler!(dummy_int_229_handler, 229);
dummy_interrupt_handler!(dummy_int_230_handler, 230);
dummy_interrupt_handler!(dummy_int_231_handler, 231);
dummy_interrupt_handler!(dummy_int_232_handler, 232);
dummy_interrupt_handler!(dummy_int_233_handler, 233);
dummy_interrupt_handler!(dummy_int_234_handler, 234);
dummy_interrupt_handler!(dummy_int_235_handler, 235);
dummy_interrupt_handler!(dummy_int_236_handler, 236);
dummy_interrupt_handler!(dummy_int_237_handler, 237);
dummy_interrupt_handler!(dummy_int_238_handler, 238);
dummy_interrupt_handler!(dummy_int_239_handler, 239);
dummy_interrupt_handler!(dummy_int_240_handler, 240);
dummy_interrupt_handler!(dummy_int_241_handler, 241);
dummy_interrupt_handler!(dummy_int_242_handler, 242);
dummy_interrupt_handler!(dummy_int_243_handler, 243);
dummy_interrupt_handler!(dummy_int_244_handler, 244);
dummy_interrupt_handler!(dummy_int_245_handler, 245);
dummy_interrupt_handler!(dummy_int_246_handler, 246);
dummy_interrupt_handler!(dummy_int_247_handler, 247);
dummy_interrupt_handler!(dummy_int_248_handler, 248);
dummy_interrupt_handler!(dummy_int_249_handler, 249);
dummy_interrupt_handler!(dummy_int_250_handler, 250);
dummy_interrupt_handler!(dummy_int_251_handler, 251);
dummy_interrupt_handler!(dummy_int_252_handler, 252);
dummy_interrupt_handler!(dummy_int_253_handler, 253);
dummy_interrupt_handler!(dummy_int_254_handler, 254);
dummy_interrupt_handler!(dummy_int_255_handler, 255);

