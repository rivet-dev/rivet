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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHandHeart.js
var require_faHandHeart = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHandHeart.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-heart";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f4bc";
    var svgPathData = "M256 0c17.7 0 32 14.3 32 32l0 208c0 8.8 7.2 16 16 16s16-7.2 16-16l0-176c0-17.7 14.3-32 32-32s32 14.3 32 32l0 176c0 8.8 7.2 16 16 16s16-7.2 16-16l0-112c0-17.7 14.3-32 32-32s32 14.3 32 32l0 208c0 97.2-78.8 176-176 176l-19.2 0c-59.6 0-116.9-22.9-160-64L12.4 341c-16-15.2-16.6-40.6-1.4-56.6s40.6-16.6 56.6-1.4l60.5 57.6c0-1.5-.1-3.1-.1-4.6l0-272c0-17.7 14.3-32 32-32s32 14.3 32 32l0 176c0 8.8 7.2 16 16 16s16-7.2 16-16l0-208c0-17.7 14.3-32 32-32zM208 362.6c0 12.1 5.1 23.6 14.1 31.6l55.2 49.7c6.1 5.5 15.3 5.5 21.4 0l55.2-49.7c9-8.1 14.1-19.6 14.1-31.6c0-23.5-19.1-42.6-42.6-42.6l-1.1 0c-14.6 0-28.3 7.3-36.4 19.5c-8.1-12.2-21.7-19.5-36.4-19.5l-1.1 0c-23.5 0-42.6 19.1-42.6 42.6z";
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
    exports.faHandHeart = exports.definition;
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

// src/_entries/faHandHeart.js
var import_faHandHeart = __toESM(require_faHandHeart());
var export_faHandHeart = import_faHandHeart.definition;
export {
  export_faHandHeart as faHandHeart
};
