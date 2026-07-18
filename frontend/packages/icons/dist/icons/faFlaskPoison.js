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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFlaskRoundPoison.js
var require_faFlaskRoundPoison = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFlaskRoundPoison.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "flask-round-poison";
    var width = 448;
    var height = 512;
    var aliases = ["flask-poison"];
    var unicode = "f6e0";
    var svgPathData = "M328.5 172.1c-5.2-3-8.5-8.4-8.5-14.4L320 64c17.7 0 32-14.3 32-32s-14.3-32-32-32L304 0 256 0 192 0 144 0 128 0C110.3 0 96 14.3 96 32s14.3 32 32 32l0 93.7c0 6-3.3 11.4-8.5 14.4c0 0 0 0 0 0c-5.3 3.1-10.4 6.4-15.4 9.9C50.8 219.7 16 281.8 16 352c0 56 22.1 106.9 58.2 144.3C84.5 507 99.3 512 114.2 512l219.6 0c15 0 29.7-5 40.1-15.7C409.9 458.9 432 408 432 352c0-76.8-41.6-143.8-103.5-179.9zM300 376.6c-7.1 5.2-12 13-12 21.8l0 9.6c0 13.3-10.7 24-24 24l-80 0c-13.3 0-24-10.7-24-24l0-9.6c0-8.8-4.9-16.7-12-21.8c-22.1-16.1-36-39.1-36-64.6c0-48.6 50.1-88 112-88s112 39.4 112 88c0 25.5-13.9 48.6-36 64.6zM184 336a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm104-24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faFlaskRoundPoison = exports.definition;
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

// src/_entries/faFlaskPoison.js
var import_faFlaskRoundPoison = __toESM(require_faFlaskRoundPoison());
var export_faFlaskPoison = import_faFlaskRoundPoison.definition;
export {
  export_faFlaskPoison as faFlaskPoison
};
