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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCartFlatbedEmpty.js
var require_faCartFlatbedEmpty = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCartFlatbedEmpty.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cart-flatbed-empty";
    var width = 640;
    var height = 512;
    var aliases = ["dolly-flatbed-empty"];
    var unicode = "f476";
    var svgPathData = "M0 32C0 14.3 14.3 0 32 0L48 0c44.2 0 80 35.8 80 80l0 288c0 8.8 7.2 16 16 16l464 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-66.7 0c1.8 5 2.7 10.4 2.7 16c0 26.5-21.5 48-48 48s-48-21.5-48-48c0-5.6 1-11 2.7-16l-197.5 0c1.8 5 2.7 10.4 2.7 16c0 26.5-21.5 48-48 48s-48-21.5-48-48c0-5.6 1-11 2.7-16L144 448c-44.2 0-80-35.8-80-80L64 80c0-8.8-7.2-16-16-16L32 64C14.3 64 0 49.7 0 32z";
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
    exports.faCartFlatbedEmpty = exports.definition;
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

// src/_entries/faCartFlatbedEmpty.js
var import_faCartFlatbedEmpty = __toESM(require_faCartFlatbedEmpty());
var export_faCartFlatbedEmpty = import_faCartFlatbedEmpty.definition;
export {
  export_faCartFlatbedEmpty as faCartFlatbedEmpty
};
