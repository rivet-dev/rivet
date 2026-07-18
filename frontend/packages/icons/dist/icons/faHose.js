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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHose.js
var require_faHose = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHose.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hose";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e419";
    var svgPathData = "M288 104c0 13.3 10.7 24 24 24s24-10.7 24-24l0-2.3 80.2-11.5C417.3 98 423.9 104 432 104c8.8 0 16-7.2 16-16l0-48c0-8.8-7.2-16-16-16c-8.1 0-14.7 6-15.8 13.7L336 26.3l0-2.3c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 80zM64 176c0-44.2 35.8-80 80-80l112 0 0-64L144 32C64.5 32 0 96.5 0 176s64.5 144 144 144l272 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-272 0c-44.2 0-80-35.8-80-80zM32 352c-17.7 0-32 14.3-32 32s14.3 32 32 32l384 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 352zm0 96c-17.7 0-32 14.3-32 32s14.3 32 32 32l384 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 448z";
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
    exports.faHose = exports.definition;
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

// src/_entries/faHose.js
var import_faHose = __toESM(require_faHose());
var export_faHose = import_faHose.definition;
export {
  export_faHose as faHose
};
