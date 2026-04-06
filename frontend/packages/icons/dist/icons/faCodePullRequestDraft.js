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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCodePullRequestDraft.js
var require_faCodePullRequestDraft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCodePullRequestDraft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "code-pull-request-draft";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e3fa";
    var svgPathData = "M56 80a24 24 0 1 1 48 0A24 24 0 1 1 56 80zm56 73.3c28.3-12.3 48-40.5 48-73.3c0-44.2-35.8-80-80-80S0 35.8 0 80c0 32.8 19.7 61 48 73.3l0 205.3C19.7 371 0 399.2 0 432c0 44.2 35.8 80 80 80s80-35.8 80-80c0-32.8-19.7-61-48-73.3l0-205.3zM56 432a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm312-24a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm0 104a80 80 0 1 0 0-160 80 80 0 1 0 0 160zM416 80a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM368 288a48 48 0 1 0 0-96 48 48 0 1 0 0 96z";
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
    exports.faCodePullRequestDraft = exports.definition;
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

// src/_entries/faCodePullRequestDraft.js
var import_faCodePullRequestDraft = __toESM(require_faCodePullRequestDraft());
var export_faCodePullRequestDraft = import_faCodePullRequestDraft.definition;
export {
  export_faCodePullRequestDraft as faCodePullRequestDraft
};
