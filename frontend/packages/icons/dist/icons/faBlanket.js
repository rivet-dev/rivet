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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBlanket.js
var require_faBlanket = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBlanket.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "blanket";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f498";
    var svgPathData = "M0 128L0 352c0 70.7 57.3 128 128 128l288 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-288 0c-35.3 0-64-28.7-64-64s28.7-64 64-64l240 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-240 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l240 0c44.2 0 80-35.8 80-80l0-176c0-53-43-96-96-96L96 32C43 32 0 75 0 128z";
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
    exports.faBlanket = exports.definition;
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

// src/_entries/faBlanket.js
var import_faBlanket = __toESM(require_faBlanket());
var export_faBlanket = import_faBlanket.definition;
export {
  export_faBlanket as faBlanket
};
