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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMoneyFromBracket.js
var require_faMoneyFromBracket = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMoneyFromBracket.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "money-from-bracket";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e312";
    var svgPathData = "M64 96l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32L0 96C0 43 43 0 96 0L544 0c53 0 96 43 96 96l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-17.7-14.3-32-32-32L96 64C78.3 64 64 78.3 64 96zm64 0l384 0 0 352c0 35.3-28.7 64-64 64l-256 0c-35.3 0-64-28.7-64-64l0-352zM256 448c0-35.3-28.7-64-64-64l0 64 64 0zm192-64c-35.3 0-64 28.7-64 64l64 0 0-64zM320 352c53 0 96-35.8 96-80s-43-80-96-80s-96 35.8-96 80s43 80 96 80z";
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
    exports.faMoneyFromBracket = exports.definition;
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

// src/_entries/faMoneyFromBracket.js
var import_faMoneyFromBracket = __toESM(require_faMoneyFromBracket());
var export_faMoneyFromBracket = import_faMoneyFromBracket.definition;
export {
  export_faMoneyFromBracket as faMoneyFromBracket
};
