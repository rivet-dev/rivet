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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyBillTrendUp.js
var require_faMoneyBillTrendUp = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyBillTrendUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "money-bill-trend-up";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e529";
    var svgPathData = "M480 8c0-13.3-10.7-24-24-24L354.2-16c-13.3 0-24 10.7-24 24s10.7 24 24 24l43.9 0-111.3 111.3-95.1-81.5c-9-7.7-22.2-7.7-31.2 0l-112 96c-10.1 8.6-11.2 23.8-2.6 33.8s23.8 11.2 33.8 2.6l96.4-82.6 96.4 82.6c9.5 8.2 23.7 7.6 32.6-1.3l127-127 0 43.9c0 13.3 10.7 24 24 24s24-10.7 24-24L480 8zM48 256c-26.5 0-48 21.5-48 48L0 464c0 26.5 21.5 48 48 48l416 0c26.5 0 48-21.5 48-48l0-160c0-26.5-21.5-48-48-48L48 256zm47.3 56c-3.4 20.1-19.2 36-39.4 39.4-4.4 .7-8-2.9-8-7.3l0-32c0-4.4 3.6-8 8-8l32 0c4.4 0 8.1 3.6 7.3 8zm0 144.1c.7 4.4-2.9 8-7.3 8l-32 0c-4.4 0-8-3.6-8-8l0-32c0-4.4 3.6-8.1 8-7.3 20.1 3.4 36 19.2 39.4 39.4zM456 416.7c4.4-.7 8 2.9 8 7.3l0 32c0 4.4-3.6 8-8 8l-32 0c-4.4 0-8.1-3.6-7.3-8 3.4-20.1 19.2-36 39.4-39.4zM416.7 312c-.7-4.4 2.9-8 7.3-8l32 0c4.4 0 8 3.6 8 8l0 32c0 4.4-3.6 8.1-8 7.3-20.1-3.4-36-19.2-39.4-39.4zM192 384a64 64 0 1 1 128 0 64 64 0 1 1 -128 0z";
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
    exports.faMoneyBillTrendUp = exports.definition;
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

// src/_entries/faMoneyBillTrendUp.js
var import_faMoneyBillTrendUp = __toESM(require_faMoneyBillTrendUp());
var export_faMoneyBillTrendUp = import_faMoneyBillTrendUp.definition;
export {
  export_faMoneyBillTrendUp as faMoneyBillTrendUp
};
