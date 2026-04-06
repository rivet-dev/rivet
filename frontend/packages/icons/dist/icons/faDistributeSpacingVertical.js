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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDistributeSpacingVertical.js
var require_faDistributeSpacingVertical = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDistributeSpacingVertical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "distribute-spacing-vertical";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e366";
    var svgPathData = "M0 56C0 42.7 10.7 32 24 32l464 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 80C10.7 80 0 69.3 0 56zM96 208c0-26.5 21.5-48 48-48l224 0c26.5 0 48 21.5 48 48l0 96c0 26.5-21.5 48-48 48l-224 0c-26.5 0-48-21.5-48-48l0-96zM24 432l464 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 480c-13.3 0-24-10.7-24-24s10.7-24 24-24z";
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
    exports.faDistributeSpacingVertical = exports.definition;
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

// src/_entries/faDistributeSpacingVertical.js
var import_faDistributeSpacingVertical = __toESM(require_faDistributeSpacingVertical());
var export_faDistributeSpacingVertical = import_faDistributeSpacingVertical.definition;
export {
  export_faDistributeSpacingVertical as faDistributeSpacingVertical
};
