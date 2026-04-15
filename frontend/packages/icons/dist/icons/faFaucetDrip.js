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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFaucetDrip.js
var require_faFaucetDrip = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFaucetDrip.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "faucet-drip";
    var width = 512;
    var height = 512;
    var aliases = [128688];
    var unicode = "e006";
    var svgPathData = "M224 32c-17.7 0-32 14.3-32 32L96 64C78.3 64 64 78.3 64 96s14.3 32 32 32l96 0 0 64-18.7 0c-8.5 0-16.6 3.4-22.6 9.4L128 224 32 224c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32l100.1 0c20.2 29 53.9 48 91.9 48s71.7-19 91.9-48l36.1 0c17.7 0 32 14.3 32 32s14.3 32 32 32l64 0c17.7 0 32-14.3 32-32 0-88.4-71.6-160-160-160l-32 0-22.6-22.6c-6-6-14.1-9.4-22.6-9.4l-18.7 0 0-64 96 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-96 0c0-17.7-14.3-32-32-32zM436.8 455.4l-18.2 42.4c-1.8 4.1-2.7 8.6-2.7 13.1l0 1.2c0 17.7 14.3 32 32 32s32-14.3 32-32l0-1.2c0-4.5-.9-8.9-2.7-13.1l-18.2-42.4c-1.9-4.5-6.3-7.4-11.2-7.4s-9.2 2.9-11.2 7.4z";
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
    exports.faFaucetDrip = exports.definition;
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

// src/_entries/faFaucetDrip.js
var import_faFaucetDrip = __toESM(require_faFaucetDrip());
var export_faFaucetDrip = import_faFaucetDrip.definition;
export {
  export_faFaucetDrip as faFaucetDrip
};
