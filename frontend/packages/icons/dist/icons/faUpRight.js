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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUpRight.js
var require_faUpRight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUpRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "up-right";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e2be";
    var svgPathData = "M336 80L96 80c-12.9 0-24.6 7.8-29.6 19.8s-2.2 25.7 6.9 34.9l72 72L11.3 340.7C4.1 347.9 0 357.8 0 368s4.1 20.1 11.3 27.3l41.4 41.4C59.9 443.9 69.8 448 80 448s20.1-4.1 27.3-11.3L241.4 302.6l72 72c9.2 9.2 22.9 11.9 34.9 6.9s19.8-16.6 19.8-29.6l0-240c0-17.7-14.3-32-32-32z";
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
    exports.faUpRight = exports.definition;
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

// src/_entries/faUpRight.js
var import_faUpRight = __toESM(require_faUpRight());
var export_faUpRight = import_faUpRight.definition;
export {
  export_faUpRight as faUpRight
};
