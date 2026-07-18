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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSimplybuilt.js
var require_faSimplybuilt = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSimplybuilt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "simplybuilt";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f215";
    var svgPathData = "M481.6 64l-106 0C361.1 64 349 75.8 349 90.3l0 39.6-185.3 0 0-39.6c0-14.5-12-26.3-26.6-26.3l-106 0C16.5 64 4.7 75.8 4.7 90.3l0 331.4c0 14.5 11.8 26.3 26.6 26.3l450.4 0c14.8 0 26.6-11.8 26.6-26.3l0-331.4c-.2-14.5-12-26.3-26.7-26.3zM150.5 222.7a66.5 66.5 0 1 1 -.3 133.1 66.5 66.5 0 1 1 .3-133.1zm211.9 .1a66.5 66.5 0 1 1 .3 132.9 66.5 66.5 0 1 1 -.3-132.9z";
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
    exports.faSimplybuilt = exports.definition;
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

// src/_entries/faSimplybuilt.js
var import_faSimplybuilt = __toESM(require_faSimplybuilt());
var export_faSimplybuilt = import_faSimplybuilt.definition;
export {
  export_faSimplybuilt as faSimplybuilt
};
