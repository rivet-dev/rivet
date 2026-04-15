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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyBillWheat.js
var require_faMoneyBillWheat = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyBillWheat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "money-bill-wheat";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e52a";
    var svgPathData = "M176-16c44.2 0 80 35.8 80 80 0 8.8-7.2 16-16 16-44.2 0-80-35.8-80-80 0-8.8 7.2-16 16-16zM56 0l48 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L56 48C42.7 48 32 37.3 32 24S42.7 0 56 0zM24 72l112 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 120C10.7 120 0 109.3 0 96S10.7 72 24 72zm8 96c0-13.3 10.7-24 24-24l48 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-48 0c-13.3 0-24-10.7-24-24zM272 0c0-8.8 7.2-16 16-16 44.2 0 80 35.8 80 80 0 8.8-7.2 16-16 16-44.2 0-80-35.8-80-80zM400-16c44.2 0 80 35.8 80 80 0 8.8-7.2 16-16 16-44.2 0-80-35.8-80-80 0-8.8 7.2-16 16-16zm80 144c0 44.2-35.8 80-80 80-8.8 0-16-7.2-16-16 0-44.2 35.8-80 80-80 8.8 0 16 7.2 16 16zM352 112c8.8 0 16 7.2 16 16 0 44.2-35.8 80-80 80-8.8 0-16-7.2-16-16 0-44.2 35.8-80 80-80zm-96 16c0 44.2-35.8 80-80 80-8.8 0-16-7.2-16-16 0-44.2 35.8-80 80-80 8.8 0 16 7.2 16 16zM0 304c0-26.5 21.5-48 48-48l416 0c26.5 0 48 21.5 48 48l0 160c0 26.5-21.5 48-48 48L48 512c-26.5 0-48-21.5-48-48L0 304zM48 416l0 48 48 0c0-26.5-21.5-48-48-48zM96 304l-48 0 0 48c26.5 0 48-21.5 48-48zM464 416c-26.5 0-48 21.5-48 48l48 0 0-48zM416 304c0 26.5 21.5 48 48 48l0-48-48 0zm-96 80a64 64 0 1 0 -128 0 64 64 0 1 0 128 0z";
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
    exports.faMoneyBillWheat = exports.definition;
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

// src/_entries/faMoneyBillWheat.js
var import_faMoneyBillWheat = __toESM(require_faMoneyBillWheat());
var export_faMoneyBillWheat = import_faMoneyBillWheat.definition;
export {
  export_faMoneyBillWheat as faMoneyBillWheat
};
