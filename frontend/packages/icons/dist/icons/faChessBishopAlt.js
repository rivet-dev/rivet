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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChessBishopPiece.js
var require_faChessBishopPiece = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChessBishopPiece.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chess-bishop-piece";
    var width = 256;
    var height = 512;
    var aliases = ["chess-bishop-alt"];
    var unicode = "f43b";
    var svgPathData = "M104 32C90.7 32 80 42.7 80 56c0 12.2 9.1 22.3 20.9 23.8C79.6 100.1 48 139 48 192c0 26.3 13.6 40.5 26.3 48L72 240c-13.3 0-24 10.7-24 24s10.7 24 24 24l5.7 0L64 384l128 0-13.7-96 5.7 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-2.3 0c12.7-7.5 26.3-21.7 26.3-48c0-22.5-5.7-42.4-13.9-59.5l-38.8 38.8c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6L177 104.4c-7.4-10-15.2-18.2-21.9-24.6C166.9 78.3 176 68.2 176 56c0-13.3-10.7-24-24-24l-48 0zM48 416L4.8 473.6C1.7 477.8 0 482.8 0 488c0 13.3 10.7 24 24 24l208 0c13.3 0 24-10.7 24-24c0-5.2-1.7-10.2-4.8-14.4L208 416 48 416z";
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
    exports.faChessBishopPiece = exports.definition;
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

// src/_entries/faChessBishopAlt.js
var import_faChessBishopPiece = __toESM(require_faChessBishopPiece());
var export_faChessBishopAlt = import_faChessBishopPiece.definition;
export {
  export_faChessBishopAlt as faChessBishopAlt
};
