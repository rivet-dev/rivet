var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUsbDrive.js
var require_faUsbDrive = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUsbDrive.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "usb-drive";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8e9";
    var svgPathData = "M64 96C28.7 96 0 124.7 0 160L0 352c0 35.3 28.7 64 64 64l352 0c17.7 0 32-14.3 32-32l0-256c0-17.7-14.3-32-32-32L64 96zm544 32l-128 0 0 256 128 0c17.7 0 32-14.3 32-32l0-192c0-17.7-14.3-32-32-32zM560 232a24 24 0 1 1 0-48 24 24 0 1 1 0 48zm24 72a24 24 0 1 1 -48 0 24 24 0 1 1 48 0z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faUsbDrive = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faUsbDrive.js
var import_faUsbDrive = __toESM(require_faUsbDrive());
var export_faUsbDrive = import_faUsbDrive.definition;
export {
  export_faUsbDrive as faUsbDrive
};
