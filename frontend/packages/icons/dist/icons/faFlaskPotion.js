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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFlaskRoundPotion.js
var require_faFlaskRoundPotion = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFlaskRoundPotion.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "flask-round-potion";
    var width = 448;
    var height = 512;
    var aliases = ["flask-potion"];
    var unicode = "f6e1";
    var svgPathData = "M192 64l64 0 0 93.7c0 30.4 16.9 56.1 40.3 69.7c43 25 71.6 71.4 71.7 124.4c-58.6-1.7-85.4-12.6-109.9-22.6c-22-9-42.2-17.2-82.1-17.2c-42.3 0-73.4 9.2-94.5 18.8c6.5-44.1 33-81.7 70.2-103.4c23.4-13.6 40.3-39.3 40.3-69.7L192 64zm128 93.7L320 64c17.7 0 32-14.3 32-32s-14.3-32-32-32L304 0 144 0 128 0C110.3 0 96 14.3 96 32s14.3 32 32 32l0 93.7c0 6-3.3 11.4-8.5 14.4C57.6 208.2 16 275.2 16 352c0 56 22.1 106.9 58.2 144.3C84.5 507 99.3 512 114.2 512l219.6 0c15 0 29.7-5 40.1-15.7C409.9 458.9 432 408 432 352c0-76.8-41.6-143.8-103.5-179.9c-5.2-3-8.5-8.4-8.5-14.4z";
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
    exports.faFlaskRoundPotion = exports.definition;
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

// src/_entries/faFlaskPotion.js
var import_faFlaskRoundPotion = __toESM(require_faFlaskRoundPotion());
var export_faFlaskPotion = import_faFlaskRoundPotion.definition;
export {
  export_faFlaskPotion as faFlaskPotion
};
