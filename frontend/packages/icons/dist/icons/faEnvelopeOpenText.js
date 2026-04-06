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

// src/node_modules/@fortawesome/free-solid-svg-icons/faEnvelopeOpenText.js
var require_faEnvelopeOpenText = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faEnvelopeOpenText.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "envelope-open-text";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f658";
    var svgPathData = "M288 33.9L96.4 175.8 254.5 293c5.3 3.9 11.2 6.9 17.5 8.7L272 464c0 5.5 .5 10.8 1.3 16L96 480c-35.3 0-64-28.7-64-64l0-239.9c0-20.3 9.6-39.4 25.9-51.4L254.5-21c9.7-7.2 21.4-11 33.5-11s23.8 3.9 33.5 11L518.1 124.7c7.2 5.3 13.1 12 17.4 19.6-2.5-.2-5-.3-7.5-.3L436.6 144 288 33.9zM320 240c0-26.5 21.5-48 48-48l160 0c26.5 0 48 21.5 48 48l0 224c0 26.5-21.5 48-48 48l-160 0c-26.5 0-48-21.5-48-48l0-224zm80 16c-13.3 0-24 10.7-24 24s10.7 24 24 24l96 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-96 0zm0 96c-13.3 0-24 10.7-24 24s10.7 24 24 24l56 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-56 0z";
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
    exports.faEnvelopeOpenText = exports.definition;
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

// src/_entries/faEnvelopeOpenText.js
var import_faEnvelopeOpenText = __toESM(require_faEnvelopeOpenText());
var export_faEnvelopeOpenText = import_faEnvelopeOpenText.definition;
export {
  export_faEnvelopeOpenText as faEnvelopeOpenText
};
