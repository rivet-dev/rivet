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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFaucet.js
var require_faFaucet = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFaucet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "faucet";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e005";
    var svgPathData = "M192 64c0-17.7 14.3-32 32-32s32 14.3 32 32l96 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-96 0 0 64 18.7 0c8.5 0 16.6 3.4 22.6 9.4l22.6 22.6 32 0c88.4 0 160 71.6 160 160 0 17.7-14.3 32-32 32l-64 0c-17.7 0-32-14.3-32-32s-14.3-32-32-32l-36.1 0c-20.2 29-53.9 48-91.9 48s-71.7-19-91.9-48L32 352c-17.7 0-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32l96 0 22.6-22.6c6-6 14.1-9.4 22.6-9.4l18.7 0 0-64-96 0c-17.7 0-32-14.3-32-32S78.3 64 96 64l96 0z";
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
    exports.faFaucet = exports.definition;
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

// src/_entries/faFaucet.js
var import_faFaucet = __toESM(require_faFaucet());
var export_faFaucet = import_faFaucet.definition;
export {
  export_faFaucet as faFaucet
};
