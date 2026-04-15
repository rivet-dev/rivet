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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGiftCard.js
var require_faGiftCard = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGiftCard.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "gift-card";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f663";
    var svgPathData = "M353.5 68.8C361.1 55.9 374.9 48 389.8 48l2.2 0c22.1 0 40 17.9 40 40s-17.9 40-40 40l-72 0-1.3 0 34.8-59.2zm-131 0L257.3 128l-1.3 0-72 0c-22.1 0-40-17.9-40-40s17.9-40 40-40l2.2 0c14.9 0 28.8 7.9 36.3 20.8zm89.6-24.3L288 85.5l-24.1-41C247.7 16.9 218.1 0 186.2 0L184 0C135.4 0 96 39.4 96 88c0 14.4 3.5 28 9.6 40L64 128c-35.3 0-64 28.7-64 64L0 448c0 35.3 28.7 64 64 64l448 0c35.3 0 64-28.7 64-64l0-256c0-35.3-28.7-64-64-64l-41.6 0c6.1-12 9.6-25.6 9.6-40c0-48.6-39.4-88-88-88l-2.2 0c-31.9 0-61.5 16.9-77.7 44.4zM512 384l0 64L64 448l0-64 448 0zm0-64L64 320l0-128 166.1 0-40.5 48.6c-8.5 10.2-7.1 25.3 3.1 33.8s25.3 7.1 33.8-3.1L288 197.5l61.6 73.9c8.5 10.2 23.6 11.6 33.8 3.1s11.6-23.6 3.1-33.8L345.9 192 512 192l0 128z";
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
    exports.faGiftCard = exports.definition;
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

// src/_entries/faGiftCard.js
var import_faGiftCard = __toESM(require_faGiftCard());
var export_faGiftCard = import_faGiftCard.definition;
export {
  export_faGiftCard as faGiftCard
};
