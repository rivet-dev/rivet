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

// src/node_modules/@fortawesome/free-solid-svg-icons/faYenSign.js
var require_faYenSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faYenSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "yen-sign";
    var width = 384;
    var height = 512;
    var aliases = [165, "cny", "jpy", "rmb", "yen"];
    var unicode = "f157";
    var svgPathData = "M74.9 46.7c-9.6-14.9-29.4-19.2-44.2-9.6S11.5 66.4 21.1 81.3L143.7 272 88 272c-13.3 0-24 10.7-24 24s10.7 24 24 24l72 0 0 32-72 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l72 0 0 48c0 17.7 14.3 32 32 32s32-14.3 32-32l0-48 72 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-72 0 0-32 72 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-55.7 0 122.6-190.7c9.6-14.9 5.3-34.7-9.6-44.2s-34.7-5.3-44.2 9.6L192 228.8 74.9 46.7z";
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
    exports.faYenSign = exports.definition;
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

// src/_entries/faRmb.js
var import_faYenSign = __toESM(require_faYenSign());
var export_faRmb = import_faYenSign.definition;
export {
  export_faRmb as faRmb
};
