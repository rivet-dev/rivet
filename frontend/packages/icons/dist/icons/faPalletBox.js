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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPalletBox.js
var require_faPalletBox = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPalletBox.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pallet-box";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e208";
    var svgPathData = "M176 0l80 0 0 112c0 5.9 3.2 11.3 8.5 14.1s11.5 2.5 16.4-.8L320 99.2l39.1 26.1c4.9 3.3 11.2 3.6 16.4 .8s8.5-8.2 8.5-14.1L384 0l80 0c26.5 0 48 21.5 48 48l0 160c0 26.5-21.5 48-48 48l-288 0c-26.5 0-48-21.5-48-48l0-160c0-26.5 21.5-48 48-48zM0 352c0-17.7 14.3-32 32-32l64 0 224 0 224 0 64 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-32 0 0 64 32 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-64 0-224 0L96 512l-64 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l32 0 0-64-32 0c-17.7 0-32-14.3-32-32zm128 96l160 0 0-64-160 0 0 64zm224 0l160 0 0-64-160 0 0 64z";
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
    exports.faPalletBox = exports.definition;
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

// src/_entries/faPalletBox.js
var import_faPalletBox = __toESM(require_faPalletBox());
var export_faPalletBox = import_faPalletBox.definition;
export {
  export_faPalletBox as faPalletBox
};
