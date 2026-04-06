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

// src/node_modules/@fortawesome/free-brands-svg-icons/faAlipay.js
var require_faAlipay = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faAlipay.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "alipay";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f642";
    var svgPathData = "M377.7 32L70.3 32C31.4 32 0 63.4 0 102.3L0 409.7C0 448.6 31.4 480 70.3 480l307.5 0c38.5 0 69.8-31.1 70.3-69.6-46-25.6-110.6-60.3-171.6-88.4-32.1 44-84.1 81-148.6 81-70.6 0-93.7-45.3-97-76.4-4-39 14.9-81.5 99.5-81.5 35.4 0 79.4 10.2 127.1 25 16.5-30.1 26.5-60.3 26.5-60.3l-178.2 0 0-16.7 92.1 0 0-31.2-109.4 0 0-19 109.4 0 0-50.4 50.9 0 0 50.4 109.4 0 0 19-109.4 0 0 31.2 88.8 0s-15.2 46.6-38.3 90.9c48.9 16.7 100 36 148.6 52.7l0-234.4c.2-38.7-31.2-70.3-69.9-70.3zM47.3 323c1 20.2 10.2 53.7 69.9 53.7 52.1 0 92.6-39.7 117.9-72.9-44.6-18.7-84.5-31.4-109.4-31.4-67.4 0-79.4 33.1-78.4 50.6z";
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
    exports.faAlipay = exports.definition;
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

// src/_entries/faAlipay.js
var import_faAlipay = __toESM(require_faAlipay());
var export_faAlipay = import_faAlipay.definition;
export {
  export_faAlipay as faAlipay
};
