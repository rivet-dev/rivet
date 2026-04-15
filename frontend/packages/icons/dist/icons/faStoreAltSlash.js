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

// src/node_modules/@fortawesome/free-solid-svg-icons/faShopSlash.js
var require_faShopSlash = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faShopSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shop-slash";
    var width = 576;
    var height = 512;
    var aliases = ["store-alt-slash"];
    var unicode = "e070";
    var svgPathData = "M41-24.9c-9.4-9.4-24.6-9.4-33.9 0S-2.3-.3 7 9.1l528 528c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-57-57 0-190.3c26.8-1.9 48-24.3 48-51.6 0-8-1.9-16-5.5-23.2L497.7 67.4C486.8 45.7 464.7 32 440.4 32L135.6 32c-10.7 0-21 2.7-30.2 7.6L41-24.9zM321.8 256L448 256 448 382.2 321.8 256zM352 421.8l-64-64 0 10.2c0 8.8-7.2 16-16 16l-128 0c-8.8 0-16-7.2-16-16l0-112 58.2 0-134.8-134.8-29.9 59.9c-3.6 7.2-5.5 15.1-5.5 23.2 0 27.3 21.2 49.7 48 51.6L64 432c0 26.5 21.5 48 48 48l192 0c26.5 0 48-21.5 48-48l0-10.2z";
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
    exports.faShopSlash = exports.definition;
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

// src/_entries/faStoreAltSlash.js
var import_faShopSlash = __toESM(require_faShopSlash());
var export_faStoreAltSlash = import_faShopSlash.definition;
export {
  export_faStoreAltSlash as faStoreAltSlash
};
