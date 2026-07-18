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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBarcodeScan.js
var require_faBarcodeScan = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBarcodeScan.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "barcode-scan";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f465";
    var svgPathData = "M88 32C74.7 32 64 42.7 64 56l0 136 64 0 0-136c0-13.3-10.7-24-24-24L88 32zM64 456c0 13.3 10.7 24 24 24l16 0c13.3 0 24-10.7 24-24l0-136-64 0 0 136zm96 8c0 8.8 7.2 16 16 16s16-7.2 16-16l0-144-32 0 0 144zm64-8c0 13.3 10.7 24 24 24l16 0c13.3 0 24-10.7 24-24l0-136-64 0 0 136zm0-264l64 0 0-136c0-13.3-10.7-24-24-24l-16 0c-13.3 0-24 10.7-24 24l0 136zm96 264c0 13.3 10.7 24 24 24l16 0c13.3 0 24-10.7 24-24l0-136-64 0 0 136zm0-264l64 0 0-136c0-13.3-10.7-24-24-24l-16 0c-13.3 0-24 10.7-24 24l0 136zM448 464c0 8.8 7.2 16 16 16s16-7.2 16-16l0-144-32 0 0 144zm0-416l0 144 32 0 0-144c0-8.8-7.2-16-16-16s-16 7.2-16 16zm64 408c0 13.3 10.7 24 24 24l16 0c13.3 0 24-10.7 24-24l0-136-64 0 0 136zm0-264l64 0 0-136c0-13.3-10.7-24-24-24l-16 0c-13.3 0-24 10.7-24 24l0 136zm-352 0l32 0 0-144c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 144zM24 232c-13.3 0-24 10.7-24 24s10.7 24 24 24l592 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L24 232z";
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
    exports.faBarcodeScan = exports.definition;
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

// src/_entries/faBarcodeScan.js
var import_faBarcodeScan = __toESM(require_faBarcodeScan());
var export_faBarcodeScan = import_faBarcodeScan.definition;
export {
  export_faBarcodeScan as faBarcodeScan
};
