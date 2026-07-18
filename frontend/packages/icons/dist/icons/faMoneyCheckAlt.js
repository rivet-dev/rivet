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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyCheckDollar.js
var require_faMoneyCheckDollar = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyCheckDollar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "money-check-dollar";
    var width = 512;
    var height = 512;
    var aliases = ["money-check-alt"];
    var unicode = "f53d";
    var svgPathData = "M64 64C28.7 64 0 92.7 0 128L0 384c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-256c0-35.3-28.7-64-64-64L64 64zM296 288l128 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-128 0c-13.3 0-24-10.7-24-24s10.7-24 24-24zm-24-88c0-13.3 10.7-24 24-24l128 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-128 0c-13.3 0-24-10.7-24-24zM148 144c11 0 20 9 20 20l0 4 8 0c11 0 20 9 20 20s-9 20-20 20l-47.5 0c-6.9 0-12.5 5.6-12.5 12.5 0 6.1 4.4 11.3 10.4 12.3l41.7 7c25.3 4.2 43.9 26.1 43.9 51.8 0 26.1-19 47.7-44 51.8l0 4.7c0 11-9 20-20 20s-20-9-20-20l0-4-24 0c-11 0-20-9-20-20s9-20 20-20l55.5 0c6.9 0 12.5-5.6 12.5-12.5 0-6.1-4.4-11.3-10.4-12.3l-41.7-7C94.5 268 76 246.1 76 220.5 76 191.7 99.2 168.3 128 168l0-4c0-11 9-20 20-20z";
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
    exports.faMoneyCheckDollar = exports.definition;
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

// src/_entries/faMoneyCheckAlt.js
var import_faMoneyCheckDollar = __toESM(require_faMoneyCheckDollar());
var export_faMoneyCheckAlt = import_faMoneyCheckDollar.definition;
export {
  export_faMoneyCheckAlt as faMoneyCheckAlt
};
