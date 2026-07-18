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

// src/node_modules/@fortawesome/free-solid-svg-icons/faEnvelopesBulk.js
var require_faEnvelopesBulk = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faEnvelopesBulk.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "envelopes-bulk";
    var width = 576;
    var height = 512;
    var aliases = ["mail-bulk"];
    var unicode = "f674";
    var svgPathData = "M112 0C85.5 0 64 21.5 64 48l0 160 80 0 0-32c0-53 43-96 96-96l208 0 0-32c0-26.5-21.5-48-48-48L112 0zM240 128c-26.5 0-48 21.5-48 48l0 32 80 0c53 0 96 43 96 96l0 112 160 0c26.5 0 48-21.5 48-48l0-192c0-26.5-21.5-48-48-48l-288 0zm200 64l48 0c13.3 0 24 10.7 24 24l0 48c0 13.3-10.7 24-24 24l-48 0c-13.3 0-24-10.7-24-24l0-48c0-13.3 10.7-24 24-24zM48 256c-26.5 0-48 21.5-48 48l0 10.4 156.6 86.2c1.1 .6 2.2 .9 3.4 .9s2.4-.3 3.4-.9L320 314.4 320 304c0-26.5-21.5-48-48-48L48 256zM320 369.2L186.6 442.6c-8.1 4.5-17.3 6.8-26.6 6.8s-18.4-2.4-26.6-6.8L0 369.2 0 464c0 26.5 21.5 48 48 48l224 0c26.5 0 48-21.5 48-48l0-94.8z";
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
    exports.faEnvelopesBulk = exports.definition;
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

// src/_entries/faMailBulk.js
var import_faEnvelopesBulk = __toESM(require_faEnvelopesBulk());
var export_faMailBulk = import_faEnvelopesBulk.definition;
export {
  export_faMailBulk as faMailBulk
};
