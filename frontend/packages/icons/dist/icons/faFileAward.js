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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFileCertificate.js
var require_faFileCertificate = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFileCertificate.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-certificate";
    var width = 512;
    var height = 512;
    var aliases = ["file-award"];
    var unicode = "f5f3";
    var svgPathData = "M128 64c0-35.3 28.7-64 64-64L352 0l0 128c0 17.7 14.3 32 32 32l128 0 0 288c0 35.3-28.7 64-64 64l-226.7 0c1.8-5.1 2.7-10.5 2.7-16l0-80c1.3-.5 2.5-1 3.8-1.5c6.8-3 14.3-7.8 20.6-15.5c6.4-7.9 10.1-17.2 11.4-27.1c.5-3.6 .8-5.7 1.1-7.1c1.1-.9 2.8-2.3 5.6-4.5c19.9-15.4 27.1-42.2 17.5-65.5c-1.4-3.3-2.1-5.4-2.6-6.7c.5-1.4 1.2-3.4 2.6-6.7c9.5-23.3 2.4-50.1-17.5-65.5c-2.8-2.2-4.5-3.6-5.6-4.5c-.3-1.4-.6-3.6-1.1-7.1c-3.4-24.9-23-44.6-47.9-47.9c-3.6-.5-5.7-.8-7.1-1.1c-.9-1.1-2.3-2.8-4.5-5.6c-15.4-19.9-42.2-27.1-65.5-17.5c-2.6 1.1-5.1 2.3-6.6 3l-.1 .1L128 64zm384 64l-128 0L384 0 512 128zM109.2 161.6L125 168c1.9 .8 4.1 .8 6.1 0l15.8-6.5c10-4.1 21.5-1 28.1 7.5l10.5 13.5c1.3 1.7 3.2 2.7 5.2 3l16.9 2.3c10.7 1.5 19.1 9.9 20.5 20.5l2.3 16.9c.3 2.1 1.4 4 3 5.2l13.5 10.5c8.5 6.6 11.6 18.1 7.5 28.1L248 285c-.8 1.9-.8 4.1 0 6.1l6.5 15.8c4.1 10 1 21.5-7.5 28.1l-13.5 10.5c-1.7 1.3-2.7 3.2-3 5.2l-2.3 16.9c-1.5 10.7-9.9 19.1-20.5 20.6L192 390.2 192 496c0 5.9-3.2 11.3-8.5 14.1s-11.5 2.5-16.4-.8L128 483.2 88.9 509.3c-4.9 3.3-11.2 3.6-16.4 .8s-8.5-8.2-8.5-14.1l0-105.8-15.5-2.1c-10.7-1.5-19.1-9.9-20.5-20.6l-2.3-16.9c-.3-2.1-1.4-4-3-5.2L9.1 334.9c-8.5-6.6-11.6-18.1-7.5-28.1L8 291c.8-1.9 .8-4.1 0-6.1L1.6 269.2c-4.1-10-1-21.5 7.5-28.1l13.5-10.5c1.7-1.3 2.7-3.2 3-5.2l2.3-16.9c1.5-10.7 9.9-19.1 20.5-20.5l16.9-2.3c2.1-.3 4-1.4 5.2-3l10.5-13.5c6.6-8.5 18.1-11.6 28.1-7.5zM192 288A64 64 0 1 0 64 288a64 64 0 1 0 128 0z";
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
    exports.faFileCertificate = exports.definition;
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

// src/_entries/faFileAward.js
var import_faFileCertificate = __toESM(require_faFileCertificate());
var export_faFileAward = import_faFileCertificate.definition;
export {
  export_faFileAward as faFileAward
};
