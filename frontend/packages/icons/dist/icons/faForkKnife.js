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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faForkKnife.js
var require_faForkKnife = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faForkKnife.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fork-knife";
    var width = 576;
    var height = 512;
    var aliases = ["utensils-alt"];
    var unicode = "f2e6";
    var svgPathData = "M475.3 27.3c5.8-5.8 6.3-15.1 1.1-21.5S462-1.9 455.1 2.7l-106.5 71C320.8 92.3 304 123.6 304 157.1c0 19.3 5.5 37.9 15.7 53.9L73.4 457.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L365 256.3c16 10.2 34.7 15.7 53.9 15.7c33.5 0 64.8-16.8 83.4-44.6l71-106.5c4.6-6.8 3.2-16-3.2-21.2s-15.6-4.8-21.5 1.1L457.4 192c-5.2 5.2-13.6 5.2-18.8 0c-4.9-4.9-5.2-12.8-.6-18.1l86.1-99.4c5.5-6.3 5.2-15.9-.8-21.8s-15.4-6.3-21.8-.8L402.1 138c-5.3 4.6-13.2 4.3-18.1-.6c-5.2-5.2-5.2-13.6 0-18.8l91.3-91.3zM52.5 7.3C47.9 2.6 41.5 0 34.9 0c-11.2 0-21 7.5-23.5 18.4C6.7 38.6 0 71.5 0 96c0 83.3 48.2 130.5 128.9 209.4c6.5 6.4 13.3 13 20.3 19.9c1.9 1.8 3.8 3.5 5.9 5.1L265.4 220.1 52.5 7.3zM457.4 502.6c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3L355.9 310.6l-45.3 45.3L457.4 502.6z";
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
    exports.faForkKnife = exports.definition;
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

// src/_entries/faForkKnife.js
var import_faForkKnife = __toESM(require_faForkKnife());
var export_faForkKnife = import_faForkKnife.definition;
export {
  export_faForkKnife as faForkKnife
};
