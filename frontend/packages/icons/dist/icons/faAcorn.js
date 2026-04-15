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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAcorn.js
var require_faAcorn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAcorn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "acorn";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f6ae";
    var svgPathData = "M262.4 4.8c10.6 8 12.8 23 4.8 33.6l-4.8 6.4c-4.4 5.9-7.9 12.4-10.3 19.2L352 64c53 0 96 43 96 96c0 17.7-14.3 32-32 32L32 192c-17.7 0-32-14.3-32-32c0-53 43-96 96-96l106.4 0C206 46.7 213.3 30.3 224 16l4.8-6.4c8-10.6 23-12.8 33.6-4.8zM213.9 476.4C78.5 425.9 48 302.1 48 224l176 0 176 0c0 78.1-30.5 201.9-165.9 252.4c-6.5 2.4-13.7 2.4-20.2 0z";
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
    exports.faAcorn = exports.definition;
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

// src/_entries/faAcorn.js
var import_faAcorn = __toESM(require_faAcorn());
var export_faAcorn = import_faAcorn.definition;
export {
  export_faAcorn as faAcorn
};
