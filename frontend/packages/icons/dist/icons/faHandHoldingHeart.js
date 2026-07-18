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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandHoldingHeart.js
var require_faHandHoldingHeart = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandHoldingHeart.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-holding-heart";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f4be";
    var svgPathData = "M279.6 31C265.5 11.5 242.9 0 218.9 0 177.5 0 144 33.5 144 74.9l0 2.4c0 64.4 82 133.4 122.2 163.3 13 9.7 30.5 9.7 43.5 0 40.2-30 122.2-98.9 122.2-163.3l0-2.4c0-41.4-33.5-74.9-74.9-74.9-24 0-46.6 11.5-60.7 31L288 42.7 279.6 31zM109.3 341.5L66.7 384 32 384c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32l320.5 0c29 0 57.3-9.3 80.7-26.5l126.6-93.3c17.8-13.1 21.6-38.1 8.5-55.9s-38.1-21.6-55.9-8.5L392.6 416 280 416c-13.3 0-24-10.7-24-24s10.7-24 24-24l72 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-152.2 0c-33.9 0-66.5 13.5-90.5 37.5z";
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
    exports.faHandHoldingHeart = exports.definition;
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

// src/_entries/faHandHoldingHeart.js
var import_faHandHoldingHeart = __toESM(require_faHandHoldingHeart());
var export_faHandHoldingHeart = import_faHandHoldingHeart.definition;
export {
  export_faHandHoldingHeart as faHandHoldingHeart
};
