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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyBill1.js
var require_faMoneyBill1 = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyBill1.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "money-bill-1";
    var width = 512;
    var height = 512;
    var aliases = ["money-bill-alt"];
    var unicode = "f3d1";
    var svgPathData = "M64 64C28.7 64 0 92.7 0 128L0 384c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-256c0-35.3-28.7-64-64-64L64 64zm192 80a112 112 0 1 1 0 224 112 112 0 1 1 0-224zM64 184l0-48c0-4.4 3.6-8 8-8l48 0c4.4 0 8.1 3.6 7.5 8-3.6 29-26.6 51.9-55.5 55.5-4.4 .5-8-3.1-8-7.5zm0 144c0-4.4 3.6-8.1 8-7.5 29 3.6 51.9 26.6 55.5 55.5 .5 4.4-3.1 8-7.5 8l-48 0c-4.4 0-8-3.6-8-8l0-48zM440 191.5c-29-3.6-51.9-26.6-55.5-55.5-.5-4.4 3.1-8 7.5-8l48 0c4.4 0 8 3.6 8 8l0 48c0 4.4-3.6 8.1-8 7.5zM448 328l0 48c0 4.4-3.6 8-8 8l-48 0c-4.4 0-8.1-3.6-7.5-8 3.6-29 26.6-51.9 55.5-55.5 4.4-.5 8 3.1 8 7.5zM240 188c-11 0-20 9-20 20 0 9.7 6.9 17.7 16 19.6l0 48.4-4 0c-11 0-20 9-20 20s9 20 20 20l48 0c11 0 20-9 20-20s-9-20-20-20l-4 0 0-68c0-11-9-20-20-20l-16 0z";
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
    exports.faMoneyBill1 = exports.definition;
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

// src/_entries/faMoneyBill1.js
var import_faMoneyBill1 = __toESM(require_faMoneyBill1());
var export_faMoneyBill1 = import_faMoneyBill1.definition;
export {
  export_faMoneyBill1 as faMoneyBill1
};
