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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDistributeSpacingHorizontal.js
var require_faDistributeSpacingHorizontal = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDistributeSpacingHorizontal.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "distribute-spacing-horizontal";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e365";
    var svgPathData = "M32 24C32 10.7 42.7 0 56 0S80 10.7 80 24l0 464c0 13.3-10.7 24-24 24s-24-10.7-24-24L32 24zM160 144c0-26.5 21.5-48 48-48l96 0c26.5 0 48 21.5 48 48l0 224c0 26.5-21.5 48-48 48l-96 0c-26.5 0-48-21.5-48-48l0-224zM456 0c13.3 0 24 10.7 24 24l0 464c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-464c0-13.3 10.7-24 24-24z";
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
    exports.faDistributeSpacingHorizontal = exports.definition;
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

// src/_entries/faDistributeSpacingHorizontal.js
var import_faDistributeSpacingHorizontal = __toESM(require_faDistributeSpacingHorizontal());
var export_faDistributeSpacingHorizontal = import_faDistributeSpacingHorizontal.definition;
export {
  export_faDistributeSpacingHorizontal as faDistributeSpacingHorizontal
};
