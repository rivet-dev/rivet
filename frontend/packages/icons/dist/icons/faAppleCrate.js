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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCrateApple.js
var require_faCrateApple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCrateApple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "crate-apple";
    var width = 512;
    var height = 512;
    var aliases = ["apple-crate"];
    var unicode = "f6b1";
    var svgPathData = "M160 54.9C160 24.6 184.6 0 214.9 0c5 0 9.1 4.1 9.1 9.1C224 39.4 199.4 64 169.1 64c-5 0-9.1-4.1-9.1-9.1zM32 171.4C32 127.8 52.4 80 96 80c15.6 0 34.1 5.9 47.2 11c10.7 4.2 22.8 4.2 33.5 0c13.1-5.1 31.6-11 47.2-11c43.6 0 64 47.8 64 91.4c0 7-.4 13.9-1.2 20.6L33.2 192c-.8-6.7-1.2-13.5-1.2-20.6zM319 192c.7-6.7 1-13.6 1-20.6c0-26.1-6-55.1-20.3-78.6c-2.7-4.4-5.7-8.7-9-12.7c15 .6 32.2 6.1 44.6 11c10.7 4.2 22.8 4.2 33.5 0c13.1-5.1 31.6-11 47.2-11c43.6 0 64 47.8 64 91.4c0 7-.4 13.9-1.2 20.6L319 192zM406.9 0c5 0 9.1 4.1 9.1 9.1C416 39.4 391.4 64 361.1 64c-5 0-9.1-4.1-9.1-9.1C352 24.6 376.6 0 406.9 0zM0 272c0-26.5 21.5-48 48-48l416 0c26.5 0 48 21.5 48 48l0 80L0 352l0-80zM0 384l512 0 0 80c0 26.5-21.5 48-48 48L48 512c-26.5 0-48-21.5-48-48l0-80zm80-96a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM64 464a16 16 0 1 0 0-32 16 16 0 1 0 0 32zM464 288a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM448 464a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faCrateApple = exports.definition;
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

// src/_entries/faAppleCrate.js
var import_faCrateApple = __toESM(require_faCrateApple());
var export_faAppleCrate = import_faCrateApple.definition;
export {
  export_faAppleCrate as faAppleCrate
};
