use crc32fast::Hasher;

// Given state of CRC32 calculation over a COBS-framed stream, returns
// two new u8 to append to the stream and a new CRC32.  None of these new bytes
// will be 00 or FF and they comprise both a valid COBS-extension of the original
// stream and a valid CRC32 of the post-cobs-encoded data.
//
// When receiving this augmented COBS frame, the CRC can be checked normally
// on the data prior to COBS decode.  Then the COBS data can be decoded in the 
// usual way.  Finally, the final 6 bytes can be discarded.  (These are the CRC
// and the two preceeding bytes that ensured COBS and CRC validity.)
fn cobs_crc32(crc32: u32) -> ([u8; 2], u32)
{
    let mut new_bytes = [6, 0];
    let mut new_crc32 = 0;

    // The loop over all possible added bytes seems inelegant and possibly slow.
    // But exhaustive testing shows that the needed byte is found in 1 iteration
    // most of the time (chances are 248/256).  And the maximum number of iterations
    // needed is 5.  (Confirmed by checking all 2^32 states of the input CRC.)
    for added_byte in 1..=255 {
        let mut hasher = Hasher::new_with_initial(crc32);
        new_bytes[1] = added_byte;
        hasher.update(&new_bytes);
        new_crc32 = hasher.finalize();
        if (new_crc32 & 0xFF000000 == 0x00000000) ||
           (new_crc32 & 0x00FF0000 == 0x00000000) ||
           (new_crc32 & 0x0000FF00 == 0x00000000) ||
           (new_crc32 & 0x000000FF == 0x00000000) ||
           (new_crc32 & 0xFF000000 == 0xFF000000) ||
           (new_crc32 & 0x00FF0000 == 0x00FF0000) ||
           (new_crc32 & 0x0000FF00 == 0x0000FF00) ||
           (new_crc32 & 0x000000FF == 0x000000FF) {
            // Bad added byte
            continue;
        }

        // The added byte fixed it.
        new_bytes[1] = added_byte;
        if added_byte > 5 {
            println!("Solved the CRC with {added_byte}");
        }
        break;
    }

    (new_bytes, new_crc32)
}

#[cfg(test)]

mod test {

    use crate::cobs_crc32::cobs_crc32;
    use byteorder::{BigEndian, ByteOrder, LittleEndian};
    use crc32fast::Hasher;

    #[test]
    // Just verifying that I can properly check CRCs computed with crc32fast.
    fn test0() {
        let message = "Hello world";
        let pad = [0; 4];
        let mut hasher = Hasher::new();

        hasher.update(message.as_bytes());
        // hasher.update(&pad);

        let crc = hasher.finalize();
        println!("Starting CRC is {crc:08x}");

        let mut crc_buf: [u8; 4] = [0; 4];
        LittleEndian::write_u32(&mut crc_buf, crc^0xFFFFFFFF);

        let mut hasher2 = Hasher::new_with_initial(crc);
        // hasher2.update(message.as_bytes());
        hasher2.update(&crc_buf);
        let crc2 = hasher2.finalize()^0xFFFFFFFF;
        println!("CRC including the final CRC value: {crc2:08x}");

        assert_eq!(crc2, 0);

    }
    
#[test]
    // Test cobs_crc32 on "Hello world"
    fn test1() {
        let message = "Hello world";
        let mut hasher = Hasher::new();

        hasher.update(message.as_bytes());
        let crc32 = hasher.finalize();
        println!("Starting CRC is {crc32:08x}");

        let (addition, new_crc) = cobs_crc32(crc32);
        println!("Additions: {:02x}, {:02x}, CRC32: {:08x}", 
            addition[0], addition[1], new_crc);

        // Re-do CRC calculation with the additional bytes.
        let mut hasher2 = Hasher::new();
        hasher2.update(message.as_bytes());
        hasher2.update(&addition[0..2]);
        let final_crc = hasher2.finalize();
        println!("Final CRC is {final_crc:08x}");

        assert_eq!(final_crc, new_crc);

    }

    // Test cobs_crc32 on "Hello World" augmented with a lot of different u32 values
    #[test]
    fn test_lots() {
        for n in 0..100_000_000 {
            let mut buf: [u8; 4] = [0; 4];
            LittleEndian::write_u32(&mut buf, n);

            let message = "Hello world";
            let mut hasher = Hasher::new();

            hasher.update(message.as_bytes());
            hasher.update(&buf);
            let crc32 = hasher.finalize();
            // println!("Starting CRC is {crc32:08x}");

            let (addition, new_crc) = cobs_crc32(crc32);
            // println!("Additions: {:02x}, {:02x}, CRC32: {:08x}", 
            //    addition[0], addition[1], new_crc);

            // Re-do CRC calculation with the additional bytes.
            let mut hasher2 = Hasher::new();
            hasher2.update(message.as_bytes());
            hasher2.update(&buf);
            hasher2.update(&addition[0..2]);
            let final_crc = hasher2.finalize();
            // println!("Final CRC is {final_crc:08x}");

            assert_eq!(final_crc, new_crc);
            assert_ne!(addition[0], 0);
            assert_ne!(addition[1], 0);
            assert_ne!(addition[0], 0xFF);
            assert_ne!(addition[1], 0xFF);
            assert_ne!(new_crc & 0xFF000000, 0x00000000);
            assert_ne!(new_crc & 0xFF000000, 0xFF000000);
            assert_ne!(new_crc & 0x00FF0000, 0x00000000);
            assert_ne!(new_crc & 0x00FF0000, 0x00FF0000);
            assert_ne!(new_crc & 0x0000FF00, 0x00000000);
            assert_ne!(new_crc & 0x0000FF00, 0x0000FF00);
            assert_ne!(new_crc & 0x000000FF, 0x00000000);
            assert_ne!(new_crc & 0x000000FF, 0x000000FF);
        }

    }

    #[test]
    // Test cobs_crc32 with all possible crc32 states.
    fn test_all() {
        for n in 0..=0xFFFFFFFF {
            let mut buf: [u8; 4] = [0; 4];
            LittleEndian::write_u32(&mut buf, n);

            let message = "Hello world";
            let mut hasher = Hasher::new();

            hasher.update(message.as_bytes());
            hasher.update(&buf);
            let crc32 = hasher.finalize();
            // println!("Starting CRC is {crc32:08x}");

            let (addition, new_crc) = cobs_crc32(crc32);
            // println!("Additions: {:02x}, {:02x}, CRC32: {:08x}", 
            //    addition[0], addition[1], new_crc);

            // Re-do CRC calculation with the additional bytes.
            let mut hasher2 = Hasher::new();
            hasher2.update(message.as_bytes());
            hasher2.update(&buf);
            hasher2.update(&addition[0..2]);
            let final_crc = hasher2.finalize();
            // println!("Final CRC is {final_crc:08x}");

            assert_eq!(final_crc, new_crc);
            assert_ne!(addition[0], 0);
            assert_ne!(addition[1], 0);
            assert_ne!(addition[0], 0xFF);
            assert_ne!(addition[1], 0xFF);
            assert_ne!(new_crc & 0xFF000000, 0x00000000);
            assert_ne!(new_crc & 0xFF000000, 0xFF000000);
            assert_ne!(new_crc & 0x00FF0000, 0x00000000);
            assert_ne!(new_crc & 0x00FF0000, 0x00FF0000);
            assert_ne!(new_crc & 0x0000FF00, 0x00000000);
            assert_ne!(new_crc & 0x0000FF00, 0x0000FF00);
            assert_ne!(new_crc & 0x000000FF, 0x00000000);
            assert_ne!(new_crc & 0x000000FF, 0x000000FF);
        }

        println!("Done all 32-bit combinations.");

    }
}

