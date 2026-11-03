// SPDX-License-Identifier: GPL-2.0-only
// Linux transport for the original MICommonInterface 32-byte fan protocol.
#include <linux/acpi.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/uaccess.h>

#define MIFS_GUID "B60BFB48-3E5B-49E4-A0E9-8CFFE1B3434B"
#define MIFS_CALL _IOWR('J', 1, unsigned char[32])
static DEFINE_MUTEX(mifs_lock);

static long mifs_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
    u8 bytes[32];
    struct acpi_buffer input = { sizeof(bytes), bytes };
    struct acpi_buffer output = { ACPI_ALLOCATE_BUFFER, NULL };
    union acpi_object *object;
    acpi_status status;
    int result = 0, i;
    bool get;

    if (cmd != MIFS_CALL)
        return -ENOTTY;
    if (copy_from_user(bytes, (void __user *)arg, sizeof(bytes)))
        return -EFAULT;
    get = bytes[1] == 250;
    if ((!get && bytes[1] != 251) || bytes[0] || bytes[2])
        return -EINVAL;
    for (i = 5; i < sizeof(bytes); i++)
        if (bytes[i])
            return -EINVAL;
    if (get) {
        if (bytes[4] || (bytes[3] != 8 && bytes[3] != 13 &&
                        bytes[3] != 19 && bytes[3] != 20 && bytes[3] != 21 && bytes[3] != 23))
            return -EINVAL;
    } else if (bytes[3] == 8) {
        if (bytes[4] > 2)
            return -EINVAL;
    } else if (bytes[3] == 23) {
        if (bytes[4] != 0)
            return -EINVAL;
    } else if (bytes[3] == 20) {
        if (bytes[4] > 1)
            return -EINVAL;
    } else if (bytes[3] == 21) {
        if (bytes[4] < 22 || bytes[4] > 58)
            return -EINVAL;
    } else {
        return -EINVAL;
    }

    mutex_lock(&mifs_lock);
    status = wmi_evaluate_method(MIFS_GUID, 0, 1, &input, &output);
    if (ACPI_FAILURE(status)) {
        result = -EIO;
        goto done;
    }
    object = output.pointer;
    if (!object || object->type != ACPI_TYPE_BUFFER || object->buffer.length < 32) {
        result = -EPROTO;
        goto done;
    }
    if (copy_to_user((void __user *)arg, object->buffer.pointer, sizeof(bytes)))
        result = -EFAULT;
done:
    kfree(output.pointer);
    mutex_unlock(&mifs_lock);
    return result;
}

static const struct file_operations mifs_fops = {
    .owner = THIS_MODULE,
    .unlocked_ioctl = mifs_ioctl,
    .compat_ioctl = mifs_ioctl,
};
static struct miscdevice mifs_device = {
    .minor = MISC_DYNAMIC_MINOR,
    .name = "jiaolong-mifs",
    .fops = &mifs_fops,
    .mode = 0600,
};
static int __init mifs_init(void)
{
    if (!wmi_has_guid(MIFS_GUID))
        return -ENODEV;
    return misc_register(&mifs_device);
}
static void __exit mifs_exit(void) { misc_deregister(&mifs_device); }
module_init(mifs_init);
module_exit(mifs_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("Jiaolong original MIFS fan protocol userspace transport");
